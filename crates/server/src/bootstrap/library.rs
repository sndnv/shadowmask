use ::api::dto::library::CreateLibraryRequest;
use domain::library::NewLibrary;
use domain::service::LibraryService;
use domain::user::Principal;

use super::executor::BootstrapEntityProvider;
use super::{BootstrapError, Created, backend, bootstrap_admin, require_unique};

pub struct LibraryBootstrapProvider<L> {
    libraries: L,
    admin: Principal,
}

impl<L> LibraryBootstrapProvider<L> {
    pub fn new(libraries: L) -> Self {
        Self { libraries, admin: bootstrap_admin() }
    }
}

impl<L> BootstrapEntityProvider for LibraryBootstrapProvider<L>
where
    L: LibraryService + Send + Sync,
{
    type Entity = NewLibrary;

    fn name(&self) -> &'static str {
        "libraries"
    }

    fn load(&self, value: &toml::Value) -> Result<NewLibrary, BootstrapError> {
        let request: CreateLibraryRequest = value.clone().try_into().map_err(|error| {
            BootstrapError::Invalid { entity: "library", reason: error.to_string() }
        })?;
        Ok(request.into())
    }

    fn validate(&self, entities: &[NewLibrary]) -> Result<(), BootstrapError> {
        require_unique(entities, "library", "name", |library| library.name.clone())
    }

    async fn create(&self, entity: NewLibrary) -> Result<Created, BootstrapError> {
        let existing = self
            .libraries
            .libraries(&self.admin)
            .await
            .map_err(|error| backend("library", error))?;
        if existing.iter().any(|library| library.name == entity.name) {
            return Ok(Created::Skipped);
        }
        self.libraries
            .create_library(&self.admin, entity)
            .await
            .map_err(|error| backend("library", error))?;
        Ok(Created::New)
    }

    fn render(&self, entity: &NewLibrary) -> String {
        format!(
            "name={} kind={:?} roots={:?} watcher={:?}",
            entity.name, entity.kind, entity.roots, entity.watcher
        )
    }

    fn extract_id(&self, entity: &NewLibrary) -> String {
        entity.name.clone()
    }
}

#[cfg(test)]
mod tests {
    use domain::library::{Library, LibraryId, LibraryKind, LibraryOrigin, WatcherStrategy};
    use jiff::Timestamp;
    use mocks::{
        MockCatalogRepo, MockJobStore, MockLibraryRepo, MockMetadataProvider, MockUserRepo,
    };
    use services::library::LibraryServiceImpl;

    use super::super::executor::run_one;
    use super::*;

    type LibrarySvc = LibraryServiceImpl<
        MockLibraryRepo,
        MockUserRepo,
        MockJobStore,
        MockCatalogRepo,
        MockMetadataProvider,
    >;

    fn library_service() -> (LibrarySvc, MockLibraryRepo) {
        let repo = MockLibraryRepo::new();
        let service = LibraryServiceImpl::new(
            repo.clone(),
            MockUserRepo::new(),
            MockJobStore::new(),
            MockCatalogRepo::new(),
            None::<MockMetadataProvider>,
        );
        (service, repo)
    }

    fn seeded_library(name: &str) -> Library {
        Library {
            id: LibraryId(format!("existing-{name}")),
            name: name.to_owned(),
            origin: LibraryOrigin::Local,
            kind: LibraryKind::Movie,
            roots: vec!["/media".to_owned()],
            sort_articles: Vec::new(),
            watcher: WatcherStrategy::Local,
            scan_schedule: None,
            metadata_sources: Vec::new(),
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn write(dir: &std::path::Path, body: &str) {
        std::fs::write(dir.join("libraries.toml"), body).unwrap();
    }

    #[tokio::test]
    async fn creates_configured_library() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "[[libraries]]\nname = \"Movies\"\nkind = \"movie\"\nroots = [\"/media/movies\"]\nwatcher = \"local\"\n",
        );
        let (service, _repo) = library_service();
        let provider = LibraryBootstrapProvider::new(service.clone());
        let result = run_one(&provider, dir.path()).await;
        assert_eq!(result.created, 1);
        assert_eq!(result.skipped, 0);
        let libraries = service.libraries(&bootstrap_admin()).await.unwrap();
        assert_eq!(libraries.len(), 1);
        assert_eq!(libraries[0].name, "Movies");
        assert_eq!(libraries[0].kind, LibraryKind::Movie);
        assert_eq!(libraries[0].watcher, WatcherStrategy::Local);
    }

    #[tokio::test]
    async fn skips_library_with_existing_name() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "[[libraries]]\nname = \"Movies\"\nkind = \"movie\"\nwatcher = \"local\"\n",
        );
        let (service, repo) = library_service();
        repo.insert_library(seeded_library("Movies"));
        let provider = LibraryBootstrapProvider::new(service.clone());
        let result = run_one(&provider, dir.path()).await;
        assert_eq!(result.created, 0);
        assert_eq!(result.skipped, 1);
        assert_eq!(service.libraries(&bootstrap_admin()).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn duplicate_names_fail_validation() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "[[libraries]]\nname = \"Movies\"\nkind = \"movie\"\nwatcher = \"local\"\n\n[[libraries]]\nname = \"Movies\"\nkind = \"tv\"\nwatcher = \"manual\"\n",
        );
        let (service, _repo) = library_service();
        let provider = LibraryBootstrapProvider::new(service.clone());
        let result = run_one(&provider, dir.path()).await;
        assert_eq!(result.created, 0);
        assert!(service.libraries(&bootstrap_admin()).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn unknown_watcher_fails_to_parse() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "[[libraries]]\nname = \"Movies\"\nkind = \"movie\"\nwatcher = \"telepathy\"\n",
        );
        let (service, _repo) = library_service();
        let provider = LibraryBootstrapProvider::new(service.clone());
        let result = run_one(&provider, dir.path()).await;
        assert_eq!(result.created, 0);
    }

    // Bootstrap runs before anything is serving, so a repository that is down has to
    // be reported as a backend failure rather than counted as a created entity.
    // Nothing exercised that conversion, which is why `backend()` ran in no test.
    #[tokio::test]
    async fn an_unreadable_repository_is_reported_rather_than_counted() {
        let (service, repo) = library_service();
        // The existence check reads through `list`, which `set_fail_get` does not
        // gate, so the write is the call that has to fail here.
        repo.set_fail_save();
        let provider = LibraryBootstrapProvider::new(service.clone());
        let entity = provider
            .load(
                &toml::from_str(
                    "name = \"Movies\"\nkind = \"movie\"\nroots = [\"/media/movies\"]\nwatcher = \"local\"\n",
                )
                .unwrap(),
            )
            .unwrap();

        assert!(matches!(
            provider.create(entity).await,
            Err(BootstrapError::Backend { entity: "library", .. })
        ));
    }
}

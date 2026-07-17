use domain::catalog::{TitleId, TitleRef, VersionId};
use domain::common::{Page, PageRequest};
use domain::error::LibraryError;
use domain::job::{Job, JobId, JobKind, JobPriority, JobStatus};
use domain::library::{
    DuplicateCandidate, DuplicateCandidateId, Library, LibraryId, LibraryKind, LibraryUpdate,
    NewLibrary, ResolutionStatus, ResolveCandidate, ResolveTarget, ScanState, ScanStatus,
    UnmatchedFile, UnmatchedFileId,
};
use domain::metadata::{ExternalId, MediaKind, MetadataProvider, MetadataQuery};
use domain::repository::{CatalogRepository, JobRepository, LibraryRepository, UserRepository};
use domain::service::LibraryService;
use domain::user::Principal;
use jiff::Timestamp;
use uuid::Uuid;

use super::{IngestJobPayload, MetadataJobPayload, RelinkJobPayload, parse_filename};
use crate::acl;

#[derive(Clone)]
pub struct LibraryServiceImpl<L, U, J, C, M> {
    libraries: L,
    users: U,
    jobs: J,
    catalog: C,
    provider: Option<M>,
}

impl<L, U, J, C, M> LibraryServiceImpl<L, U, J, C, M> {
    pub fn new(libraries: L, users: U, jobs: J, catalog: C, provider: Option<M>) -> Self {
        Self {
            libraries,
            users,
            jobs,
            catalog,
            provider,
        }
    }
}

impl<L, U, J, C, M> LibraryServiceImpl<L, U, J, C, M>
where
    L: LibraryRepository + Sync,
    U: UserRepository + Sync,
    J: JobRepository + Sync,
    C: CatalogRepository + Sync,
    M: MetadataProvider + Sync,
{
    async fn visible_to(&self, caller: &Principal, id: &LibraryId) -> Result<bool, LibraryError> {
        if acl::is_admin(caller) || acl::is_automation(caller) {
            return Ok(true);
        }
        let access: Vec<LibraryId> = self
            .users
            .list_library_access(&caller.user)
            .await?
            .into_iter()
            .map(|entry| entry.library)
            .collect();
        Ok(acl::can_access_library(&access, id))
    }

    async fn require_library(
        &self,
        caller: &Principal,
        id: &LibraryId,
    ) -> Result<Library, LibraryError> {
        let library = self
            .libraries
            .get(id)
            .await?
            .ok_or(LibraryError::NotFound)?;
        if self.visible_to(caller, id).await? {
            Ok(library)
        } else {
            Err(LibraryError::NotFound)
        }
    }

    async fn title_display(
        &self,
        title: &TitleId,
    ) -> Result<Option<(String, Option<u16>, MediaKind)>, LibraryError> {
        Ok(match title {
            TitleId::Movie(id) => self
                .catalog
                .get_movie(id)
                .await?
                .map(|m| (m.title, m.year, MediaKind::Movie)),
            TitleId::Episode(id) => self
                .catalog
                .get_episode(id)
                .await?
                .map(|e| (e.title, None, MediaKind::Series)),
        })
    }
}

impl<L, U, J, C, M> LibraryService for LibraryServiceImpl<L, U, J, C, M>
where
    L: LibraryRepository + Sync,
    U: UserRepository + Sync,
    J: JobRepository + Sync,
    C: CatalogRepository + Sync,
    M: MetadataProvider + Sync,
{
    async fn libraries(&self, caller: &Principal) -> Result<Vec<Library>, LibraryError> {
        let all = self.libraries.list().await?;
        if acl::is_admin(caller) {
            return Ok(all);
        }
        let access: Vec<LibraryId> = self
            .users
            .list_library_access(&caller.user)
            .await?
            .into_iter()
            .map(|entry| entry.library)
            .collect();
        Ok(all
            .into_iter()
            .filter(|library| acl::can_access_library(&access, &library.id))
            .collect())
    }

    async fn library(&self, caller: &Principal, id: &LibraryId) -> Result<Library, LibraryError> {
        self.require_library(caller, id).await
    }

    async fn create_library(
        &self,
        caller: &Principal,
        input: NewLibrary,
    ) -> Result<Library, LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        let now = Timestamp::now();
        let library = Library {
            id: LibraryId(Uuid::new_v4().to_string()),
            name: input.name,
            kind: input.kind,
            roots: input.roots,
            watcher: input.watcher,
            scan_schedule: input.scan_schedule,
            metadata_sources: input.metadata_sources,
            created_at: now,
            updated_at: now,
        };
        self.libraries.upsert(library.clone()).await?;
        Ok(library)
    }

    async fn update_library(
        &self,
        caller: &Principal,
        id: &LibraryId,
        update: LibraryUpdate,
    ) -> Result<Library, LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        self.libraries
            .get(id)
            .await?
            .ok_or(LibraryError::NotFound)?;
        let now = Timestamp::now();
        let library = Library {
            id: id.clone(),
            name: update.name,
            kind: update.kind,
            roots: update.roots,
            watcher: update.watcher,
            scan_schedule: update.scan_schedule,
            metadata_sources: update.metadata_sources,
            created_at: now,
            updated_at: now,
        };
        self.libraries.upsert(library.clone()).await?;
        Ok(library)
    }

    async fn delete_library(&self, caller: &Principal, id: &LibraryId) -> Result<(), LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        self.libraries
            .get(id)
            .await?
            .ok_or(LibraryError::NotFound)?;
        self.libraries.delete(id).await?;
        Ok(())
    }

    async fn scan_state(
        &self,
        caller: &Principal,
        id: &LibraryId,
    ) -> Result<ScanState, LibraryError> {
        self.require_library(caller, id).await?;
        Ok(self.libraries.scan_state(id).await?.unwrap_or(ScanState {
            library: id.clone(),
            status: ScanStatus::Idle,
            progress: 0.0,
            started_at: None,
            last_scanned_at: None,
            error: None,
        }))
    }

    async fn trigger_scan(&self, caller: &Principal, id: &LibraryId) -> Result<(), LibraryError> {
        self.require_library(caller, id).await?;
        if matches!(
            self.libraries.scan_state(id).await?,
            Some(state) if matches!(state.status, ScanStatus::Queued | ScanStatus::Running)
        ) {
            return Err(LibraryError::ScanInProgress);
        }
        let now = Timestamp::now();
        self.libraries
            .save_scan_state(ScanState {
                library: id.clone(),
                status: ScanStatus::Queued,
                progress: 0.0,
                started_at: None,
                last_scanned_at: None,
                error: None,
            })
            .await?;
        self.jobs
            .enqueue(Job {
                id: JobId(Uuid::new_v4().to_string()),
                kind: JobKind::LibraryScan,
                status: JobStatus::Queued,
                priority: JobPriority::Normal,
                payload: id.0.clone(),
                attempts: 0,
                progress: 0.0,
                available_at: now,
                last_error: None,
                created_at: now,
                updated_at: now,
                started_at: None,
                finished_at: None,
            })
            .await?;
        Ok(())
    }

    async fn unmatched(
        &self,
        caller: &Principal,
        id: &LibraryId,
        page: PageRequest,
    ) -> Result<Page<UnmatchedFile>, LibraryError> {
        self.require_library(caller, id).await?;
        Ok(self.libraries.list_unmatched(id, page).await?)
    }

    async fn duplicates(
        &self,
        caller: &Principal,
        id: &LibraryId,
        page: PageRequest,
    ) -> Result<Page<DuplicateCandidate>, LibraryError> {
        self.require_library(caller, id).await?;
        Ok(self.libraries.list_duplicates(id, page).await?)
    }

    async fn unmatched_candidates(
        &self,
        caller: &Principal,
        id: &LibraryId,
        unmatched: &UnmatchedFileId,
        query: Option<String>,
    ) -> Result<Vec<ResolveCandidate>, LibraryError> {
        let library = self.require_library(caller, id).await?;
        let file = self
            .libraries
            .get_unmatched(unmatched)
            .await?
            .filter(|file| file.library == *id)
            .ok_or(LibraryError::NotFound)?;
        let mut candidates = Vec::new();
        for candidate in &file.candidates {
            if let Some((title, year, kind)) = self.title_display(&candidate.title).await? {
                candidates.push(ResolveCandidate {
                    target: ResolveTarget::Existing(candidate.title.clone()),
                    title,
                    year,
                    kind,
                });
            }
        }
        if let Some(provider) = &self.provider {
            let parsed = parse_filename(&file.path);
            let kind = match library.kind {
                LibraryKind::Movie => MediaKind::Movie,
                LibraryKind::Tv => MediaKind::Series,
            };
            let search = MetadataQuery {
                title: query.unwrap_or(parsed.title),
                year: parsed.year,
                kind,
            };
            if let Ok(matches) = provider.search(&search).await {
                for candidate in matches {
                    candidates.push(ResolveCandidate {
                        target: ResolveTarget::Provider(candidate.external_id),
                        title: candidate.title,
                        year: candidate.year,
                        kind: candidate.kind,
                    });
                }
            }
        }
        Ok(candidates)
    }

    async fn resolve_unmatched(
        &self,
        caller: &Principal,
        id: &LibraryId,
        unmatched: &UnmatchedFileId,
        target: ResolveTarget,
    ) -> Result<(), LibraryError> {
        self.require_library(caller, id).await?;
        let file = self
            .libraries
            .get_unmatched(unmatched)
            .await?
            .filter(|file| file.library == *id)
            .ok_or(LibraryError::NotFound)?;
        let payload = IngestJobPayload {
            library: id.clone(),
            unmatched: unmatched.clone(),
            path: file.path,
            target,
        }
        .encode()
        .expect("ingest job payload serializes");
        let now = Timestamp::now();
        self.jobs
            .enqueue(Job {
                id: JobId(Uuid::new_v4().to_string()),
                kind: JobKind::Ingest,
                status: JobStatus::Queued,
                priority: JobPriority::Normal,
                payload,
                attempts: 0,
                progress: 0.0,
                available_at: now,
                last_error: None,
                created_at: now,
                updated_at: now,
                started_at: None,
                finished_at: None,
            })
            .await?;
        Ok(())
    }

    async fn dismiss_duplicate(
        &self,
        caller: &Principal,
        id: &LibraryId,
        duplicate: &DuplicateCandidateId,
    ) -> Result<(), LibraryError> {
        self.require_library(caller, id).await?;
        self.libraries
            .set_duplicate_status(duplicate, ResolutionStatus::Dismissed)
            .await?;
        Ok(())
    }

    async fn resolve_duplicate(
        &self,
        caller: &Principal,
        id: &LibraryId,
        duplicate: &DuplicateCandidateId,
    ) -> Result<(), LibraryError> {
        self.require_library(caller, id).await?;
        self.libraries
            .set_duplicate_status(duplicate, ResolutionStatus::Resolved)
            .await?;
        Ok(())
    }

    async fn reidentify(
        &self,
        caller: &Principal,
        title: TitleRef,
        external_id: Option<ExternalId>,
    ) -> Result<(), LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        let payload = MetadataJobPayload { title, external_id }
            .encode()
            .expect("metadata job payload serializes");
        let now = Timestamp::now();
        self.jobs
            .enqueue(Job {
                id: JobId(Uuid::new_v4().to_string()),
                kind: JobKind::Metadata,
                status: JobStatus::Queued,
                priority: JobPriority::Normal,
                payload,
                attempts: 0,
                progress: 0.0,
                available_at: now,
                last_error: None,
                created_at: now,
                updated_at: now,
                started_at: None,
                finished_at: None,
            })
            .await?;
        Ok(())
    }

    async fn relink_version(
        &self,
        caller: &Principal,
        version: &VersionId,
        target: ResolveTarget,
    ) -> Result<(), LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        let detail = self
            .catalog
            .version_detail(version)
            .await?
            .ok_or(LibraryError::NotFound)?;
        if let ResolveTarget::Existing(title) = &target {
            let exists = match title {
                TitleId::Movie(id) => self.catalog.get_movie(id).await?.is_some(),
                TitleId::Episode(id) => self.catalog.get_episode(id).await?.is_some(),
            };
            if !exists {
                return Err(LibraryError::NotFound);
            }
        }
        let payload = RelinkJobPayload {
            version: version.clone(),
            library: detail.version.library.clone(),
            path: detail.version.path.clone(),
            target,
        }
        .encode()
        .expect("relink job payload serializes");
        let now = Timestamp::now();
        self.jobs
            .enqueue(Job {
                id: JobId(Uuid::new_v4().to_string()),
                kind: JobKind::Relink,
                status: JobStatus::Queued,
                priority: JobPriority::Normal,
                payload,
                attempts: 0,
                progress: 0.0,
                available_at: now,
                last_error: None,
                created_at: now,
                updated_at: now,
                started_at: None,
                finished_at: None,
            })
            .await?;
        Ok(())
    }

    async fn jobs(&self, caller: &Principal) -> Result<Vec<Job>, LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        Ok(self.jobs.list().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{
        MockCatalogRepo, MockJobStore, MockLibraryRepo, MockMetadataProvider, MockUserRepo,
    };
    use domain::library::WatcherStrategy;
    use domain::repository::JobRepository;
    use domain::user::{Role, User, UserId};
    use jiff::Timestamp;

    type Svc = LibraryServiceImpl<
        MockLibraryRepo,
        MockUserRepo,
        MockJobStore,
        MockCatalogRepo,
        MockMetadataProvider,
    >;

    fn library(id: &str) -> Library {
        Library {
            id: LibraryId(id.into()),
            name: format!("Lib {id}"),
            kind: LibraryKind::Movie,
            roots: vec!["/media".into()],
            watcher: WatcherStrategy::Manual,
            scan_schedule: None,
            metadata_sources: Vec::new(),
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn user(id: &str) -> User {
        User {
            id: UserId(id.into()),
            username: id.into(),
            password_hash: "hash".into(),
            role: Role::User,
            max_content_rating: None,
            preferred_audio: Vec::new(),
            preferred_subtitle: Vec::new(),
            concurrent_stream_limit: None,
            bitrate_cap: None,
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn admin() -> Principal {
        Principal {
            user: UserId("admin".into()),
            role: Role::Admin,
        }
    }

    fn member() -> Principal {
        Principal {
            user: UserId("u1".into()),
            role: Role::User,
        }
    }

    fn automation() -> Principal {
        Principal {
            user: UserId("webhook".into()),
            role: Role::Automation,
        }
    }

    fn page() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 10,
        }
    }

    async fn seeded() -> Svc {
        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library("lib1"));
        libraries.insert_library(library("lib2"));
        let users = MockUserRepo::new();
        users.insert(user("u1"));
        users
            .set_library_access(&UserId("u1".into()), &[LibraryId("lib1".into())])
            .await
            .unwrap();
        LibraryServiceImpl::new(
            libraries,
            users,
            MockJobStore::new(),
            MockCatalogRepo::new(),
            None,
        )
    }

    #[tokio::test]
    async fn admin_sees_all_member_filtered() {
        let svc = seeded().await;
        assert_eq!(svc.libraries(&admin()).await.unwrap().len(), 2);
        let visible = svc.libraries(&member()).await.unwrap();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id, LibraryId("lib1".into()));
    }

    #[tokio::test]
    async fn detail_hides_disallowed_as_not_found() {
        let svc = seeded().await;
        assert!(
            svc.library(&member(), &LibraryId("lib1".into()))
                .await
                .is_ok()
        );
        assert!(matches!(
            svc.library(&member(), &LibraryId("lib2".into()))
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
        assert!(matches!(
            svc.library(&admin(), &LibraryId("missing".into()))
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn scan_state_defaults_then_trigger_enqueues_and_conflicts() {
        let svc = seeded().await;
        let id = LibraryId("lib1".into());
        let state = svc.scan_state(&admin(), &id).await.unwrap();
        assert_eq!(state.status, ScanStatus::Idle);

        svc.trigger_scan(&admin(), &id).await.unwrap();
        assert_eq!(svc.jobs.list().await.unwrap().len(), 1);
        assert_eq!(svc.jobs.list().await.unwrap()[0].kind, JobKind::LibraryScan);
        assert_eq!(
            svc.scan_state(&admin(), &id).await.unwrap().status,
            ScanStatus::Queued
        );
        assert!(matches!(
            svc.trigger_scan(&admin(), &id).await.unwrap_err(),
            LibraryError::ScanInProgress
        ));
    }

    #[tokio::test]
    async fn automation_bypasses_acl_for_scan() {
        let svc = seeded().await;
        let id = LibraryId("lib2".into());
        svc.trigger_scan(&automation(), &id).await.unwrap();
        assert_eq!(
            svc.scan_state(&automation(), &id).await.unwrap().status,
            ScanStatus::Queued
        );
    }

    #[tokio::test]
    async fn unmatched_and_duplicates_gated() {
        let svc = seeded().await;
        let id = LibraryId("lib1".into());
        assert!(
            svc.unmatched(&admin(), &id, page())
                .await
                .unwrap()
                .items
                .is_empty()
        );
        assert!(
            svc.duplicates(&admin(), &id, page())
                .await
                .unwrap()
                .items
                .is_empty()
        );
        assert!(matches!(
            svc.unmatched(&member(), &LibraryId("lib2".into()), page())
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
        assert!(matches!(
            svc.duplicates(&member(), &LibraryId("lib2".into()), page())
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn backend_errors_propagate() {
        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library("lib1"));
        libraries.set_fail_get();
        let svc = LibraryServiceImpl::new(
            libraries,
            MockUserRepo::new(),
            MockJobStore::new(),
            MockCatalogRepo::new(),
            None::<MockMetadataProvider>,
        );
        assert!(matches!(
            svc.library(&admin(), &LibraryId("lib1".into()))
                .await
                .unwrap_err(),
            LibraryError::Repository(_)
        ));
    }

    fn new_library() -> NewLibrary {
        NewLibrary {
            name: "New".into(),
            kind: LibraryKind::Movie,
            roots: vec!["/n".into()],
            watcher: WatcherStrategy::Manual,
            scan_schedule: None,
            metadata_sources: vec!["tmdb".into()],
        }
    }

    fn library_update() -> LibraryUpdate {
        LibraryUpdate {
            name: "Renamed".into(),
            kind: LibraryKind::Tv,
            roots: vec!["/n".into()],
            watcher: WatcherStrategy::Manual,
            scan_schedule: None,
            metadata_sources: Vec::new(),
        }
    }

    #[tokio::test]
    async fn create_update_delete_library() {
        let svc = seeded().await;

        let created = svc.create_library(&admin(), new_library()).await.unwrap();
        assert!(svc.library(&admin(), &created.id).await.is_ok());
        assert!(matches!(
            svc.create_library(&member(), new_library())
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));

        let updated = svc
            .update_library(&admin(), &created.id, library_update())
            .await
            .unwrap();
        assert_eq!(updated.name, "Renamed");
        assert_eq!(updated.kind, LibraryKind::Tv);
        assert!(matches!(
            svc.update_library(&admin(), &LibraryId("ghost".into()), library_update())
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
        assert!(matches!(
            svc.update_library(&member(), &created.id, library_update())
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));

        svc.delete_library(&admin(), &created.id).await.unwrap();
        assert!(matches!(
            svc.library(&admin(), &created.id).await.unwrap_err(),
            LibraryError::NotFound
        ));
        assert!(matches!(
            svc.delete_library(&admin(), &created.id).await.unwrap_err(),
            LibraryError::NotFound
        ));
        assert!(matches!(
            svc.delete_library(&member(), &LibraryId("lib1".into()))
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));
    }

    #[tokio::test]
    async fn jobs_admin_only() {
        let svc = seeded().await;
        let now = Timestamp::now();
        svc.jobs
            .enqueue(Job {
                id: JobId("j1".into()),
                kind: JobKind::LibraryScan,
                status: JobStatus::Queued,
                priority: JobPriority::Normal,
                payload: "lib1".into(),
                attempts: 0,
                progress: 0.0,
                available_at: now,
                last_error: None,
                created_at: now,
                updated_at: now,
                started_at: None,
                finished_at: None,
            })
            .await
            .unwrap();
        assert_eq!(svc.jobs(&admin()).await.unwrap().len(), 1);
        assert!(matches!(
            svc.jobs(&member()).await.unwrap_err(),
            LibraryError::Forbidden
        ));
    }

    #[tokio::test]
    async fn library_writes_propagate_backend_errors() {
        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library("lib1"));
        libraries.set_fail_save();
        let svc = LibraryServiceImpl::new(
            libraries,
            MockUserRepo::new(),
            MockJobStore::new(),
            MockCatalogRepo::new(),
            None::<MockMetadataProvider>,
        );
        assert!(matches!(
            svc.create_library(&admin(), new_library())
                .await
                .unwrap_err(),
            LibraryError::Repository(_)
        ));
        assert!(matches!(
            svc.delete_library(&admin(), &LibraryId("lib1".into()))
                .await
                .unwrap_err(),
            LibraryError::Repository(_)
        ));
    }

    #[tokio::test]
    async fn unmatched_candidates_lists_catalog_and_provider() {
        use domain::catalog::{Episode, EpisodeId, Movie, MovieId, SeasonId};
        use domain::library::{MatchCandidate, UnmatchedFileId};
        use domain::metadata::{ExternalId, MetadataMatch};

        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library("lib1"));
        let uid = UnmatchedFileId("uf1".into());
        libraries
            .insert_unmatched(UnmatchedFile {
                id: uid.clone(),
                library: LibraryId("lib1".into()),
                path: "/media/The Matrix (1999).mkv".into(),
                candidates: vec![
                    MatchCandidate {
                        title: TitleId::Movie(MovieId("m1".into())),
                        confidence: 0.9,
                        label: "Alpha".into(),
                    },
                    MatchCandidate {
                        title: TitleId::Episode(EpisodeId("e1".into())),
                        confidence: 0.5,
                        label: "Beta".into(),
                    },
                ],
                created_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .unwrap();

        let catalog = MockCatalogRepo::new();
        catalog.add_movie(Movie {
            id: MovieId("m1".into()),
            title: "The Matrix".into(),
            year: Some(1999),
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog.add_episode(Episode {
            id: EpisodeId("e1".into()),
            season: SeasonId("se1".into()),
            number: 1,
            title: "Pilot".into(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        let provider = MockMetadataProvider::with_matches(vec![MetadataMatch {
            external_id: ExternalId {
                source: "tmdb".into(),
                value: "movie/603".into(),
            },
            title: "The Matrix".into(),
            year: Some(1999),
            kind: MediaKind::Movie,
        }]);

        let svc = LibraryServiceImpl::new(
            libraries,
            MockUserRepo::new(),
            MockJobStore::new(),
            catalog,
            Some(provider),
        );
        let id = LibraryId("lib1".into());
        let candidates = svc
            .unmatched_candidates(&admin(), &id, &uid, None)
            .await
            .unwrap();
        assert_eq!(candidates.len(), 3);
        assert!(matches!(candidates[0].target, ResolveTarget::Existing(_)));
        assert_eq!(candidates[0].title, "The Matrix");
        assert_eq!(candidates[0].kind, MediaKind::Movie);
        assert_eq!(candidates[1].kind, MediaKind::Series);
        assert!(matches!(candidates[2].target, ResolveTarget::Provider(_)));

        assert!(matches!(
            svc.unmatched_candidates(&admin(), &id, &UnmatchedFileId("ghost".into()), None)
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn resolve_unmatched_enqueues_ingest_job() {
        use domain::catalog::MovieId;
        use domain::library::{UnmatchedFile, UnmatchedFileId};

        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library("lib1"));
        let uid = UnmatchedFileId("uf1".into());
        libraries
            .insert_unmatched(UnmatchedFile {
                id: uid.clone(),
                library: LibraryId("lib1".into()),
                path: "/m/x.mkv".into(),
                candidates: Vec::new(),
                created_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .unwrap();
        let jobs = MockJobStore::new();
        let svc = LibraryServiceImpl::new(
            libraries,
            MockUserRepo::new(),
            jobs.clone(),
            MockCatalogRepo::new(),
            None::<MockMetadataProvider>,
        );
        let id = LibraryId("lib1".into());
        let target = ResolveTarget::Existing(TitleId::Movie(MovieId("m1".into())));

        svc.resolve_unmatched(&admin(), &id, &uid, target.clone())
            .await
            .unwrap();
        let enqueued = jobs.list().await.unwrap();
        assert_eq!(enqueued.len(), 1);
        assert_eq!(enqueued[0].kind, JobKind::Ingest);

        assert!(matches!(
            svc.resolve_unmatched(&admin(), &id, &UnmatchedFileId("ghost".into()), target)
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn dismiss_and_resolve_duplicates_hide_them() {
        use domain::catalog::MovieId;
        use domain::library::{DuplicateCandidate, DuplicateCandidateId};

        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library("lib1"));
        let id = LibraryId("lib1".into());
        let dup = |d: &str| DuplicateCandidate {
            id: DuplicateCandidateId(d.into()),
            title: TitleId::Movie(MovieId("m1".into())),
            paths: vec!["/a.mkv".into()],
        };
        libraries.insert_duplicate(&id, dup("d1")).await.unwrap();
        libraries.insert_duplicate(&id, dup("d2")).await.unwrap();
        let svc = LibraryServiceImpl::new(
            libraries,
            MockUserRepo::new(),
            MockJobStore::new(),
            MockCatalogRepo::new(),
            None::<MockMetadataProvider>,
        );

        svc.dismiss_duplicate(&admin(), &id, &DuplicateCandidateId("d1".into()))
            .await
            .unwrap();
        svc.resolve_duplicate(&admin(), &id, &DuplicateCandidateId("d2".into()))
            .await
            .unwrap();
        assert!(
            svc.duplicates(&admin(), &id, page())
                .await
                .unwrap()
                .items
                .is_empty()
        );

        assert!(matches!(
            svc.dismiss_duplicate(
                &admin(),
                &LibraryId("ghost".into()),
                &DuplicateCandidateId("d1".into())
            )
            .await
            .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn reidentify_enqueues_metadata_job_admin_only() {
        use domain::catalog::MovieId;
        use domain::metadata::ExternalId;

        let jobs = MockJobStore::new();
        let svc = LibraryServiceImpl::new(
            MockLibraryRepo::new(),
            MockUserRepo::new(),
            jobs.clone(),
            MockCatalogRepo::new(),
            None::<MockMetadataProvider>,
        );
        let title = TitleRef::Movie(MovieId("m1".into()));

        svc.reidentify(
            &admin(),
            title.clone(),
            Some(ExternalId {
                source: "tmdb".into(),
                value: "movie/603".into(),
            }),
        )
        .await
        .unwrap();
        let enqueued = jobs.list().await.unwrap();
        assert_eq!(enqueued.len(), 1);
        assert_eq!(enqueued[0].kind, JobKind::Metadata);

        assert!(matches!(
            svc.reidentify(&member(), title, None).await.unwrap_err(),
            LibraryError::Forbidden
        ));
    }

    fn seeded_version_catalog() -> MockCatalogRepo {
        use domain::catalog::{Movie, MovieId, TitleId, Version, VersionId};
        use domain::common::Quality;
        let catalog = MockCatalogRepo::new();
        catalog.add_version(Version {
            id: VersionId("v1".into()),
            title: TitleId::Movie(MovieId("m-old".into())),
            library: LibraryId("lib1".into()),
            quality: Quality::Fhd,
            container: "mkv".into(),
            path: "/media/v1.mkv".into(),
            size_bytes: 1,
            duration_ms: 1000,
            edition: None,
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        });
        catalog.add_movie(Movie {
            id: MovieId("m-new".into()),
            title: "New".into(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog
    }

    fn relink_svc(catalog: MockCatalogRepo) -> (Svc, MockJobStore) {
        let jobs = MockJobStore::new();
        let svc = LibraryServiceImpl::new(
            MockLibraryRepo::new(),
            MockUserRepo::new(),
            jobs.clone(),
            catalog,
            None::<MockMetadataProvider>,
        );
        (svc, jobs)
    }

    #[tokio::test]
    async fn relink_existing_movie_enqueues_job_admin_only() {
        use domain::catalog::{MovieId, TitleId, VersionId};
        let (svc, jobs) = relink_svc(seeded_version_catalog());
        let target = ResolveTarget::Existing(TitleId::Movie(MovieId("m-new".into())));
        svc.relink_version(&admin(), &VersionId("v1".into()), target.clone())
            .await
            .unwrap();
        let enqueued = jobs.list().await.unwrap();
        assert_eq!(enqueued.len(), 1);
        assert_eq!(enqueued[0].kind, JobKind::Relink);
        assert!(matches!(
            svc.relink_version(&member(), &VersionId("v1".into()), target)
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));
    }

    #[tokio::test]
    async fn relink_to_existing_episode_enqueues() {
        use domain::catalog::{Episode, EpisodeId, SeasonId, TitleId, VersionId};
        let catalog = seeded_version_catalog();
        catalog.add_episode(Episode {
            id: EpisodeId("e-new".into()),
            season: SeasonId("s1".into()),
            number: 1,
            title: "Ep".into(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        let (svc, jobs) = relink_svc(catalog);
        svc.relink_version(
            &admin(),
            &VersionId("v1".into()),
            ResolveTarget::Existing(TitleId::Episode(EpisodeId("e-new".into()))),
        )
        .await
        .unwrap();
        assert_eq!(jobs.list().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn relink_provider_target_enqueues() {
        use domain::catalog::VersionId;
        use domain::metadata::ExternalId;
        let (svc, jobs) = relink_svc(seeded_version_catalog());
        svc.relink_version(
            &admin(),
            &VersionId("v1".into()),
            ResolveTarget::Provider(ExternalId {
                source: "tmdb".into(),
                value: "movie/603".into(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(jobs.list().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn relink_unknown_version_is_not_found() {
        use domain::catalog::{MovieId, TitleId, VersionId};
        let (svc, _jobs) = relink_svc(MockCatalogRepo::new());
        assert!(matches!(
            svc.relink_version(
                &admin(),
                &VersionId("ghost".into()),
                ResolveTarget::Existing(TitleId::Movie(MovieId("m-new".into()))),
            )
            .await
            .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn relink_unknown_target_is_not_found() {
        use domain::catalog::{MovieId, TitleId, VersionId};
        let (svc, _jobs) = relink_svc(seeded_version_catalog());
        assert!(matches!(
            svc.relink_version(
                &admin(),
                &VersionId("v1".into()),
                ResolveTarget::Existing(TitleId::Movie(MovieId("m-missing".into()))),
            )
            .await
            .unwrap_err(),
            LibraryError::NotFound
        ));
    }
}

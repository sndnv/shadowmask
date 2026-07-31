use domain::error::{FetchError, RepositoryError};
use domain::job::Job;
use domain::library::{DiscoveredFile, LibraryOrigin, ResolveTarget};
use domain::media::{FetchRequest, MediaFetcher, MediaProbe};
use domain::metadata::ExternalId;
use domain::repository::LibraryRepository;
use services::library::{FetchJobPayload, ResolveIngester, fetch_filename_stem};

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct FetchJobHandler<R, F, P, E> {
    repo: R,
    fetcher: F,
    probe: P,
    ingester: E,
}

impl<R, F, P, E> FetchJobHandler<R, F, P, E> {
    pub fn new(repo: R, fetcher: F, probe: P, ingester: E) -> Self {
        Self {
            repo,
            fetcher,
            probe,
            ingester,
        }
    }
}

impl<R, F, P, E> JobHandler for FetchJobHandler<R, F, P, E>
where
    R: LibraryRepository + Send + Sync,
    F: MediaFetcher + Send + Sync,
    P: MediaProbe + Send + Sync,
    E: ResolveIngester + Send + Sync,
{
    async fn handle(&self, job: &Job) -> Result<(), JobError> {
        let payload = FetchJobPayload::decode(&job.payload)
            .map_err(|e| JobError::Permanent(format!("invalid fetch payload: {e}")))?;
        let library = self
            .repo
            .get(&payload.library)
            .await
            .map_err(retryable)?
            .ok_or_else(|| {
                JobError::Permanent(format!("library not found: {}", payload.library.0))
            })?;
        if library.origin != LibraryOrigin::External {
            return Err(JobError::Permanent(format!(
                "library [{}] is not an external fetch library",
                library.id.0
            )));
        }
        let root = library.roots.first().ok_or_else(|| {
            JobError::Permanent(format!("external library [{}] has no root", library.id.0))
        })?;
        let dest_dir = format!("{root}/{}", job.id.0);
        let stem = fetch_filename_stem(
            payload.kind,
            &payload.title,
            None,
            payload.season,
            payload.episode,
        );
        tracing::info!("fetching [{}] into [{}]", payload.source_url, dest_dir);
        let fetched = self
            .fetcher
            .fetch(&FetchRequest {
                url: payload.source_url.clone(),
                dest_dir,
                filename_stem: stem,
            })
            .await
            .map_err(fetch_error)?;
        let probe = self
            .probe
            .probe(&fetched.path)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?;
        let file = DiscoveredFile {
            library: payload.library.clone(),
            path: fetched.path,
            size_bytes: fetched.size_bytes,
            subtitle_siblings: Vec::new(),
            probe,
        };
        let target = ResolveTarget::Provider(ExternalId {
            source: "imdb".to_owned(),
            value: payload.imdb_id.clone().unwrap_or_default(),
        });
        self.ingester
            .ingest_resolved(&library, &file, &target, Some(&job.id))
            .await
            .map_err(retryable)?;
        Ok(())
    }
}

fn retryable(err: RepositoryError) -> JobError {
    JobError::Retryable(err.to_string())
}

fn fetch_error(err: FetchError) -> JobError {
    match err {
        FetchError::Spawn(m) | FetchError::NoOutput(m) => JobError::Permanent(m),
        FetchError::Download(m) | FetchError::Io(m) => JobError::Retryable(m),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::common::PageRequest;
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use domain::library::{Library, LibraryId, LibraryKind, LibraryOrigin, WatcherStrategy};
    use domain::media::FetchedMedia;
    use domain::repository::{CatalogRepository, JobRepository};
    use jiff::Timestamp;
    use services::library::Enricher;
    use services::mock::{
        MockCatalogRepo, MockJobStore, MockLibraryRepo, MockMediaProbe, MockMetadataProvider,
    };

    enum Outcome {
        Ok,
        Permanent,
        Retryable,
    }

    struct MockFetcher {
        outcome: Outcome,
    }

    impl MediaFetcher for MockFetcher {
        async fn fetch(&self, request: &FetchRequest) -> Result<FetchedMedia, FetchError> {
            match self.outcome {
                Outcome::Ok => Ok(FetchedMedia {
                    path: format!("{}/{}.mkv", request.dest_dir, request.filename_stem),
                    size_bytes: 4096,
                }),
                Outcome::Permanent => Err(FetchError::NoOutput("no output".into())),
                Outcome::Retryable => Err(FetchError::Download("network".into())),
            }
        }
    }

    fn external_library() -> Library {
        Library {
            id: LibraryId("ext".into()),
            name: "Ext".into(),
            origin: LibraryOrigin::External,
            kind: LibraryKind::Movie,
            roots: vec!["/ext".into()],
            watcher: WatcherStrategy::Manual,
            scan_schedule: None,
            metadata_sources: Vec::new(),
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn local_library() -> Library {
        Library {
            origin: LibraryOrigin::Local,
            ..external_library()
        }
    }

    fn page() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 10,
        }
    }

    fn payload() -> FetchJobPayload {
        FetchJobPayload {
            library: LibraryId("ext".into()),
            source_url: "https://example.com/watch?v=abc".into(),
            kind: LibraryKind::Movie,
            title: "The Matrix".into(),
            imdb_id: None,
            season: None,
            episode: None,
        }
    }

    fn job(payload: String) -> Job {
        let now = Timestamp::UNIX_EPOCH;
        Job {
            id: JobId("job-1".into()),
            kind: JobKind::Fetch,
            status: JobStatus::Running,
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
            parent_id: None,
        }
    }

    #[allow(clippy::type_complexity)]
    fn handler(
        repo: MockLibraryRepo,
        outcome: Outcome,
        probe: MockMediaProbe,
    ) -> (
        FetchJobHandler<
            MockLibraryRepo,
            MockFetcher,
            MockMediaProbe,
            Enricher<MockCatalogRepo, MockMetadataProvider, MockJobStore, MockLibraryRepo>,
        >,
        MockCatalogRepo,
        MockJobStore,
    ) {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let ingester = Enricher::new(
            catalog.clone(),
            None::<MockMetadataProvider>,
            jobs.clone(),
            MockLibraryRepo::new(),
        );
        (
            FetchJobHandler::new(repo, MockFetcher { outcome }, probe, ingester),
            catalog,
            jobs,
        )
    }

    #[tokio::test]
    async fn fetches_ingests_and_parents_the_fanout() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(external_library());
        let (h, catalog, jobs) = handler(repo, Outcome::Ok, MockMediaProbe::new());

        h.handle(&job(payload().encode().unwrap())).await.unwrap();

        assert_eq!(
            catalog
                .list_library_versions(&LibraryId("ext".into()), page())
                .await
                .unwrap()
                .total,
            1
        );
        let enqueued = jobs.list().await.unwrap();
        assert!(!enqueued.is_empty());
        assert!(
            enqueued
                .iter()
                .all(|j| j.parent_id == Some(JobId("job-1".into())))
        );
    }

    #[tokio::test]
    async fn invalid_payload_is_permanent() {
        let (h, _, _) = handler(MockLibraryRepo::new(), Outcome::Ok, MockMediaProbe::new());
        assert!(matches!(
            h.handle(&job("garbage".into())).await.unwrap_err(),
            JobError::Permanent(_)
        ));
    }

    #[tokio::test]
    async fn missing_library_is_permanent() {
        let (h, _, _) = handler(MockLibraryRepo::new(), Outcome::Ok, MockMediaProbe::new());
        assert!(matches!(
            h.handle(&job(payload().encode().unwrap()))
                .await
                .unwrap_err(),
            JobError::Permanent(_)
        ));
    }

    #[tokio::test]
    async fn non_external_library_is_permanent() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(local_library());
        let (h, _, _) = handler(repo, Outcome::Ok, MockMediaProbe::new());
        assert!(matches!(
            h.handle(&job(payload().encode().unwrap()))
                .await
                .unwrap_err(),
            JobError::Permanent(_)
        ));
    }

    #[tokio::test]
    async fn fetch_permanent_error_is_permanent() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(external_library());
        let (h, _, _) = handler(repo, Outcome::Permanent, MockMediaProbe::new());
        assert!(matches!(
            h.handle(&job(payload().encode().unwrap()))
                .await
                .unwrap_err(),
            JobError::Permanent(_)
        ));
    }

    #[tokio::test]
    async fn fetch_retryable_error_is_retryable() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(external_library());
        let (h, _, _) = handler(repo, Outcome::Retryable, MockMediaProbe::new());
        assert!(matches!(
            h.handle(&job(payload().encode().unwrap()))
                .await
                .unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn probe_failure_is_retryable() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(external_library());
        let (h, _, _) = handler(
            repo,
            Outcome::Ok,
            MockMediaProbe::new().failing_on("/ext/job-1/The Matrix.mkv"),
        );
        assert!(matches!(
            h.handle(&job(payload().encode().unwrap()))
                .await
                .unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn library_without_root_is_permanent() {
        let repo = MockLibraryRepo::new();
        let mut lib = external_library();
        lib.roots = Vec::new();
        repo.insert_library(lib);
        let (h, _, _) = handler(repo, Outcome::Ok, MockMediaProbe::new());
        assert!(matches!(
            h.handle(&job(payload().encode().unwrap()))
                .await
                .unwrap_err(),
            JobError::Permanent(_)
        ));
    }

    #[tokio::test]
    async fn repo_get_error_is_retryable() {
        let repo = MockLibraryRepo::new();
        repo.set_fail_get();
        let (h, _, _) = handler(repo, Outcome::Ok, MockMediaProbe::new());
        assert!(matches!(
            h.handle(&job(payload().encode().unwrap()))
                .await
                .unwrap_err(),
            JobError::Retryable(_)
        ));
    }
}

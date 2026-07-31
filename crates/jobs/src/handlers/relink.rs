use domain::error::RepositoryError;
use domain::job::Job;
use domain::library::DiscoveredFile;
use domain::media::MediaProbe;
use domain::repository::LibraryRepository;
use services::library::{RelinkJobPayload, ResolveIngester};

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct RelinkJobHandler<R, P, E> {
    repo: R,
    probe: P,
    ingester: E,
}

impl<R, P, E> RelinkJobHandler<R, P, E> {
    pub fn new(repo: R, probe: P, ingester: E) -> Self {
        Self {
            repo,
            probe,
            ingester,
        }
    }
}

impl<R, P, E> JobHandler for RelinkJobHandler<R, P, E>
where
    R: LibraryRepository + Send + Sync,
    P: MediaProbe + Send + Sync,
    E: ResolveIngester + Send + Sync,
{
    async fn handle(&self, job: &Job) -> Result<(), JobError> {
        let payload = RelinkJobPayload::decode(&job.payload)
            .map_err(|e| JobError::Permanent(format!("invalid relink payload: {e}")))?;
        let library = self
            .repo
            .get(&payload.library)
            .await
            .map_err(retryable)?
            .ok_or_else(|| {
                JobError::Permanent(format!("library not found: {}", payload.library.0))
            })?;
        tracing::info!(
            "relinking [{}] in library [{}]",
            payload.path,
            payload.library.0
        );
        let probe = self
            .probe
            .probe(&payload.path)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?;
        let file = DiscoveredFile {
            library: payload.library.clone(),
            path: payload.path.clone(),
            size_bytes: 0,
            subtitle_siblings: Vec::new(),
            probe,
        };
        self.ingester
            .ingest_resolved(&library, &file, &payload.target, Some(&job.id))
            .await
            .map_err(retryable)?;
        Ok(())
    }
}

fn retryable(err: RepositoryError) -> JobError {
    JobError::Retryable(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{MovieId, TitleId, VersionId};
    use domain::common::PageRequest;
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use domain::library::{
        Library, LibraryId, LibraryKind, LibraryOrigin, ResolveTarget, WatcherStrategy,
    };
    use domain::repository::CatalogRepository;
    use jiff::Timestamp;
    use services::library::Enricher;
    use services::mock::{
        MockCatalogRepo, MockJobStore, MockLibraryRepo, MockMediaProbe, MockMetadataProvider,
    };

    fn library() -> Library {
        Library {
            id: LibraryId("lib".into()),
            name: "Lib".into(),
            origin: LibraryOrigin::Local,
            kind: LibraryKind::Movie,
            roots: vec!["/m".into()],
            watcher: WatcherStrategy::Manual,
            scan_schedule: None,
            metadata_sources: Vec::new(),
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn page() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 10,
        }
    }

    fn ingester() -> Enricher<MockCatalogRepo, MockMetadataProvider, MockJobStore, MockLibraryRepo>
    {
        Enricher::new(
            MockCatalogRepo::new(),
            None,
            MockJobStore::new(),
            MockLibraryRepo::new(),
        )
    }

    fn job(payload: String) -> Job {
        let now = Timestamp::UNIX_EPOCH;
        Job {
            id: JobId("job-1".into()),
            kind: JobKind::Relink,
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

    fn payload(path: &str, library: &str) -> RelinkJobPayload {
        RelinkJobPayload {
            version: VersionId("v1".into()),
            library: LibraryId(library.into()),
            path: path.into(),
            target: ResolveTarget::Existing(TitleId::Movie(MovieId("m-new".into()))),
        }
    }

    #[tokio::test]
    async fn relinks_version_to_target_title() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library());
        let catalog = MockCatalogRepo::new();
        let handler = RelinkJobHandler::new(
            repo,
            MockMediaProbe::new(),
            Enricher::new(
                catalog.clone(),
                None::<MockMetadataProvider>,
                MockJobStore::new(),
                MockLibraryRepo::new(),
            ),
        );

        handler
            .handle(&job(payload("/m/x.mkv", "lib").encode().unwrap()))
            .await
            .unwrap();

        let versions = catalog
            .list_library_versions(&LibraryId("lib".into()), page())
            .await
            .unwrap();
        assert_eq!(versions.total, 1);
        assert_eq!(
            versions.items[0].title,
            TitleId::Movie(MovieId("m-new".into()))
        );
    }

    #[tokio::test]
    async fn invalid_payload_is_permanent() {
        let handler =
            RelinkJobHandler::new(MockLibraryRepo::new(), MockMediaProbe::new(), ingester());
        assert!(matches!(
            handler.handle(&job("garbage".into())).await.unwrap_err(),
            JobError::Permanent(_)
        ));
    }

    #[tokio::test]
    async fn missing_library_is_permanent() {
        let handler =
            RelinkJobHandler::new(MockLibraryRepo::new(), MockMediaProbe::new(), ingester());
        assert!(matches!(
            handler
                .handle(&job(payload("/m/x.mkv", "nope").encode().unwrap()))
                .await
                .unwrap_err(),
            JobError::Permanent(_)
        ));
    }

    #[tokio::test]
    async fn probe_failure_is_retryable() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library());
        let handler = RelinkJobHandler::new(
            repo,
            MockMediaProbe::new().failing_on("/m/x.mkv"),
            ingester(),
        );
        assert!(matches!(
            handler
                .handle(&job(payload("/m/x.mkv", "lib").encode().unwrap()))
                .await
                .unwrap_err(),
            JobError::Retryable(_)
        ));
    }
}

use domain::error::RepositoryError;
use domain::job::Job;
use domain::library::{DiscoveredFile, ResolutionStatus};
use domain::media::MediaProbe;
use domain::repository::LibraryRepository;
use services::library::{IngestJobPayload, ResolveIngester, ResolveOutcome};

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct IngestJobHandler<R, P, E> {
    repo: R,
    probe: P,
    ingester: E,
}

impl<R, P, E> IngestJobHandler<R, P, E> {
    pub fn new(repo: R, probe: P, ingester: E) -> Self {
        Self {
            repo,
            probe,
            ingester,
        }
    }
}

impl<R, P, E> JobHandler for IngestJobHandler<R, P, E>
where
    R: LibraryRepository + Send + Sync,
    P: MediaProbe + Send + Sync,
    E: ResolveIngester + Send + Sync,
{
    async fn handle(&self, job: &Job) -> Result<(), JobError> {
        let payload = IngestJobPayload::decode(&job.payload)
            .map_err(|e| JobError::Permanent(format!("invalid ingest payload: {e}")))?;
        let library = self
            .repo
            .get(&payload.library)
            .await
            .map_err(retryable)?
            .ok_or_else(|| {
                JobError::Permanent(format!("library not found: {}", payload.library.0))
            })?;
        tracing::info!(
            "ingesting [{}] into library [{}]",
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
        let outcome = self
            .ingester
            .ingest_resolved(&library, &file, &payload.target, Some(&job.id))
            .await
            .map_err(retryable)?;
        if outcome == ResolveOutcome::Unidentified {
            return Err(JobError::Retryable(format!(
                "the provider returned no metadata for [{}]; the file was ingested but the title was not identified",
                payload.path
            )));
        }
        self.repo
            .set_unmatched_status(&payload.unmatched, ResolutionStatus::Resolved)
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
    use domain::catalog::{MovieId, TitleId};
    use domain::common::PageRequest;
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use domain::library::{
        Library, LibraryId, LibraryKind, LibraryOrigin, ResolveTarget, UnmatchedFile,
        UnmatchedFileId, WatcherStrategy,
    };
    use domain::repository::CatalogRepository;
    use jiff::Timestamp;
    use mocks::{
        MockCatalogRepo, MockJobStore, MockLibraryRepo, MockMediaProbe, MockMetadataProvider,
    };
    use services::library::Enricher;

    fn library() -> Library {
        Library {
            id: LibraryId("lib".into()),
            name: "Lib".into(),
            origin: LibraryOrigin::Local,
            kind: LibraryKind::Movie,
            roots: vec!["/m".into()],
            sort_articles: Vec::new(),
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
            kind: JobKind::Ingest,
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

    fn payload(path: &str, library: &str) -> IngestJobPayload {
        IngestJobPayload {
            library: LibraryId(library.into()),
            unmatched: UnmatchedFileId("uf1".into()),
            path: path.into(),
            target: ResolveTarget::Existing(TitleId::Movie(MovieId("m1".into()))),
        }
    }

    #[tokio::test]
    async fn ingests_file_and_marks_row_resolved() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library());
        repo.insert_unmatched(UnmatchedFile {
            id: UnmatchedFileId("uf1".into()),
            library: LibraryId("lib".into()),
            path: "/m/x.mkv".into(),
            candidates: Vec::new(),
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        })
        .await
        .unwrap();
        let catalog = MockCatalogRepo::new();
        let handler = IngestJobHandler::new(
            repo.clone(),
            MockMediaProbe::new(),
            Enricher::new(
                catalog.clone(),
                None::<MockMetadataProvider>,
                MockJobStore::new(),
                MockLibraryRepo::new(),
            ),
        );

        handler
            .handle(&job(payload("/m/x.mkv", "lib").encode()))
            .await
            .unwrap();

        assert_eq!(
            catalog
                .list_library_versions(&LibraryId("lib".into()), page())
                .await
                .unwrap()
                .total,
            1
        );
        assert!(
            repo.list_unmatched(&LibraryId("lib".into()), page())
                .await
                .unwrap()
                .items
                .is_empty()
        );
    }

    #[tokio::test]
    async fn invalid_payload_is_permanent() {
        let handler =
            IngestJobHandler::new(MockLibraryRepo::new(), MockMediaProbe::new(), ingester());
        assert!(matches!(
            handler.handle(&job("garbage".into())).await.unwrap_err(),
            JobError::Permanent(_)
        ));
    }

    #[tokio::test]
    async fn missing_library_is_permanent() {
        let handler =
            IngestJobHandler::new(MockLibraryRepo::new(), MockMediaProbe::new(), ingester());
        assert!(matches!(
            handler
                .handle(&job(payload("/m/x.mkv", "nope").encode()))
                .await
                .unwrap_err(),
            JobError::Permanent(_)
        ));
    }

    #[tokio::test]
    async fn a_backend_failure_reading_the_library_is_retryable() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library());
        repo.set_fail_get();
        let handler = IngestJobHandler::new(repo, MockMediaProbe::new(), ingester());

        assert!(
            matches!(
                handler
                    .handle(&job(payload("/m/x.mkv", "lib").encode()))
                    .await
                    .unwrap_err(),
                JobError::Retryable(_)
            ),
            "a database blip must not permanently drop the file"
        );
    }

    #[tokio::test]
    async fn probe_failure_is_retryable() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library());
        let handler = IngestJobHandler::new(
            repo,
            MockMediaProbe::new().failing_on("/m/x.mkv"),
            ingester(),
        );
        assert!(matches!(
            handler
                .handle(&job(payload("/m/x.mkv", "lib").encode()))
                .await
                .unwrap_err(),
            JobError::Retryable(_)
        ));
    }
}

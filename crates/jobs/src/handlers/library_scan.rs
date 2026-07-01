use domain::error::{RepositoryError, WalkError};
use domain::job::Job;
use domain::library::{LibraryId, ScanState, ScanStatus, SourceWalker};
use domain::media::MediaProbe;
use domain::repository::LibraryRepository;
use jiff::Timestamp;
use services::library::Scanner;

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct LibraryScanHandler<R, W, P> {
    repo: R,
    scanner: Scanner<W, P>,
}

impl<R, W, P> LibraryScanHandler<R, W, P> {
    pub fn new(repo: R, scanner: Scanner<W, P>) -> Self {
        Self { repo, scanner }
    }
}

impl<R, W, P> JobHandler for LibraryScanHandler<R, W, P>
where
    R: LibraryRepository + Send + Sync,
    W: SourceWalker + Send + Sync,
    P: MediaProbe + Send + Sync,
{
    async fn handle(&self, job: &Job) -> Result<(), JobError> {
        if job.payload.is_empty() {
            return Err(JobError::Permanent(
                "library scan job has empty payload".to_owned(),
            ));
        }
        let id = LibraryId(job.payload.clone());
        let library = self
            .repo
            .get(&id)
            .await
            .map_err(retryable)?
            .ok_or_else(|| JobError::Permanent(format!("library not found: {}", id.0)))?;

        if matches!(
            self.repo.scan_state(&id).await.map_err(retryable)?,
            Some(state) if state.status == ScanStatus::Running
        ) {
            return Ok(());
        }

        self.repo
            .save_scan_state(running(&id))
            .await
            .map_err(retryable)?;

        match self.scanner.scan(&library).await {
            Ok(_report) => {
                self.repo
                    .save_scan_state(idle(&id, Timestamp::now()))
                    .await
                    .map_err(retryable)?;
                Ok(())
            }
            Err(err) => {
                self.repo
                    .save_scan_state(failed(&id, &err))
                    .await
                    .map_err(retryable)?;
                let message = err.to_string();
                Err(match err {
                    WalkError::RootNotFound(_) => JobError::Permanent(message),
                    WalkError::Unreadable(_) => JobError::Retryable(message),
                })
            }
        }
    }
}

fn retryable(err: RepositoryError) -> JobError {
    JobError::Retryable(err.to_string())
}

fn running(id: &LibraryId) -> ScanState {
    ScanState {
        library: id.clone(),
        status: ScanStatus::Running,
        progress: 0.0,
        last_scanned_at: None,
        error: None,
    }
}

fn idle(id: &LibraryId, now: Timestamp) -> ScanState {
    ScanState {
        library: id.clone(),
        status: ScanStatus::Idle,
        progress: 1.0,
        last_scanned_at: Some(now),
        error: None,
    }
}

fn failed(id: &LibraryId, err: &WalkError) -> ScanState {
    ScanState {
        library: id.clone(),
        status: ScanStatus::Failed,
        progress: 0.0,
        last_scanned_at: None,
        error: Some(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use domain::library::{Library, LibraryKind, WalkedEntry, WatcherStrategy};
    use services::mock::{MockLibraryRepo, MockMediaProbe, MockSourceWalker};

    fn library(roots: &[&str]) -> Library {
        Library {
            id: LibraryId("lib".into()),
            name: "Lib".into(),
            kind: LibraryKind::Movie,
            roots: roots.iter().map(|r| (*r).into()).collect(),
            watcher: WatcherStrategy::Manual,
            scan_schedule: None,
            metadata_sources: Vec::new(),
        }
    }

    fn scan_job(payload: &str) -> Job {
        let now = Timestamp::now();
        Job {
            id: JobId("job-1".into()),
            kind: JobKind::LibraryScan,
            status: JobStatus::Running,
            priority: JobPriority::Normal,
            payload: payload.to_owned(),
            attempts: 0,
            progress: 0.0,
            available_at: now,
            last_error: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn handler(
        repo: MockLibraryRepo,
        walker: MockSourceWalker,
        probe: MockMediaProbe,
    ) -> LibraryScanHandler<MockLibraryRepo, MockSourceWalker, MockMediaProbe> {
        LibraryScanHandler::new(repo, Scanner::new(walker, probe))
    }

    #[tokio::test]
    async fn happy_path_marks_idle_with_timestamp() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library(&["/m"]));
        let walker = MockSourceWalker::new().with_entries(
            "/m",
            vec![WalkedEntry {
                path: "/m/a.mkv".into(),
                size_bytes: 1,
            }],
        );
        let handler = handler(repo.clone(), walker, MockMediaProbe::new());

        handler.handle(&scan_job("lib")).await.unwrap();

        let state = repo
            .scan_state(&LibraryId("lib".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(state.status, ScanStatus::Idle);
        assert_eq!(state.progress, 1.0);
        assert!(state.last_scanned_at.is_some());
    }

    #[tokio::test]
    async fn empty_payload_is_permanent() {
        let handler = handler(
            MockLibraryRepo::new(),
            MockSourceWalker::new(),
            MockMediaProbe::new(),
        );
        assert!(matches!(
            handler.handle(&scan_job("")).await.unwrap_err(),
            JobError::Permanent(_)
        ));
    }

    #[tokio::test]
    async fn missing_library_is_permanent() {
        let handler = handler(
            MockLibraryRepo::new(),
            MockSourceWalker::new(),
            MockMediaProbe::new(),
        );
        assert!(matches!(
            handler.handle(&scan_job("nope")).await.unwrap_err(),
            JobError::Permanent(_)
        ));
    }

    #[tokio::test]
    async fn already_running_is_a_benign_skip() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library(&["/m"]));
        repo.save_scan_state(running(&LibraryId("lib".into())))
            .await
            .unwrap();
        let handler = handler(repo.clone(), MockSourceWalker::new(), MockMediaProbe::new());

        handler.handle(&scan_job("lib")).await.unwrap();

        let state = repo
            .scan_state(&LibraryId("lib".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(state.status, ScanStatus::Running);
    }

    #[tokio::test]
    async fn root_not_found_fails_permanently() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library(&["/missing"]));
        let walker = MockSourceWalker::new().with_failing("/missing");
        let handler = handler(repo.clone(), walker, MockMediaProbe::new());

        assert!(matches!(
            handler.handle(&scan_job("lib")).await.unwrap_err(),
            JobError::Permanent(_)
        ));
        let state = repo
            .scan_state(&LibraryId("lib".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(state.status, ScanStatus::Failed);
        assert!(state.error.is_some());
    }

    #[tokio::test]
    async fn unreadable_root_is_retryable() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library(&["/x"]));
        let walker = MockSourceWalker::new().with_unreadable("/x");
        let handler = handler(repo.clone(), walker, MockMediaProbe::new());

        assert!(matches!(
            handler.handle(&scan_job("lib")).await.unwrap_err(),
            JobError::Retryable(_)
        ));
        let state = repo
            .scan_state(&LibraryId("lib".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(state.status, ScanStatus::Failed);
    }

    #[tokio::test]
    async fn repo_get_error_is_retryable() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library(&["/m"]));
        repo.set_fail_get();
        let handler = handler(repo, MockSourceWalker::new(), MockMediaProbe::new());

        assert!(matches!(
            handler.handle(&scan_job("lib")).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn repo_save_error_is_retryable() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library(&["/m"]));
        repo.set_fail_save();
        let handler = handler(repo, MockSourceWalker::new(), MockMediaProbe::new());

        assert!(matches!(
            handler.handle(&scan_job("lib")).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }
}

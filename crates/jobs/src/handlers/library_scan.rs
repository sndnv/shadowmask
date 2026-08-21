use domain::error::{RepositoryError, WalkError};
use domain::job::Job;
use domain::library::{LibraryId, ScanState, ScanStatus, SourceWalker};
use domain::media::MediaProbe;
use domain::repository::LibraryRepository;
use jiff::Timestamp;
use services::library::{ScanEnricher, Scanner};

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct LibraryScanHandler<R, W, P, E> {
    repo: R,
    scanner: Scanner<W, P>,
    enricher: E,
}

impl<R, W, P, E> LibraryScanHandler<R, W, P, E> {
    pub fn new(repo: R, scanner: Scanner<W, P>, enricher: E) -> Self {
        Self {
            repo,
            scanner,
            enricher,
        }
    }
}

impl<R, W, P, E> JobHandler for LibraryScanHandler<R, W, P, E>
where
    R: LibraryRepository + Send + Sync,
    W: SourceWalker + Send + Sync,
    P: MediaProbe + Send + Sync,
    E: ScanEnricher + Send + Sync,
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
        tracing::info!("scanning library [{}]", id.0);

        if matches!(
            self.repo.scan_state(&id).await.map_err(retryable)?,
            Some(state) if state.status == ScanStatus::Running
        ) {
            return Ok(());
        }

        let started = Timestamp::now();
        self.repo
            .save_scan_state(running(&id, started))
            .await
            .map_err(retryable)?;

        let repo = &self.repo;
        let scanned = self
            .scanner
            .scan(&library, |done, total| {
                let state = probing(&id, started, done, total);
                async move {
                    if let Err(err) = repo.save_scan_state(state).await {
                        tracing::warn!("could not report scan progress: {err}");
                    }
                }
            })
            .await;

        match scanned {
            Ok(report) => {
                self.repo
                    .save_scan_state(idle(&id, started, Timestamp::now()))
                    .await
                    .map_err(retryable)?;
                self.enricher.enrich(&library, &report, Some(&job.id)).await;
                Ok(())
            }
            Err(err) => {
                self.repo
                    .save_scan_state(failed(&id, started, &err))
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

fn running(id: &LibraryId, started: Timestamp) -> ScanState {
    ScanState {
        library: id.clone(),
        status: ScanStatus::Running,
        progress: 0.0,
        started_at: Some(started),
        last_scanned_at: None,
        error: None,
    }
}

fn probing(id: &LibraryId, started: Timestamp, done: u32, total: u32) -> ScanState {
    ScanState {
        library: id.clone(),
        status: ScanStatus::Running,
        progress: f64::from(done) as f32 / f64::from(total.max(1)) as f32,
        started_at: Some(started),
        last_scanned_at: None,
        error: None,
    }
}

fn idle(id: &LibraryId, started: Timestamp, now: Timestamp) -> ScanState {
    ScanState {
        library: id.clone(),
        status: ScanStatus::Idle,
        progress: 1.0,
        started_at: Some(started),
        last_scanned_at: Some(now),
        error: None,
    }
}

fn failed(id: &LibraryId, started: Timestamp, err: &WalkError) -> ScanState {
    ScanState {
        library: id.clone(),
        status: ScanStatus::Failed,
        progress: 0.0,
        started_at: Some(started),
        last_scanned_at: None,
        error: Some(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::*;
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use domain::library::{
        Library, LibraryKind, LibraryOrigin, ScanReport, WalkedEntry, WatcherStrategy,
    };
    use mocks::{MockLibraryRepo, MockMediaProbe, MockSourceWalker};
    use services::library::NoopEnricher;

    #[derive(Clone)]
    struct SpyEnricher {
        called: Arc<AtomicBool>,
    }

    impl ScanEnricher for SpyEnricher {
        async fn enrich(&self, _library: &Library, _report: &ScanReport, _parent: Option<&JobId>) {
            self.called.store(true, Ordering::Relaxed);
        }
    }

    fn library(roots: &[&str]) -> Library {
        Library {
            id: LibraryId("lib".into()),
            name: "Lib".into(),
            origin: LibraryOrigin::Local,
            kind: LibraryKind::Movie,
            roots: roots.iter().map(|r| (*r).into()).collect(),
            sort_articles: Vec::new(),
            watcher: WatcherStrategy::Manual,
            scan_schedule: None,
            metadata_sources: Vec::new(),
            created_at: Timestamp::now(),
            updated_at: Timestamp::now(),
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
            started_at: None,
            finished_at: None,
            parent_id: None,
        }
    }

    fn handler(
        repo: MockLibraryRepo,
        walker: MockSourceWalker,
        probe: MockMediaProbe,
    ) -> LibraryScanHandler<MockLibraryRepo, MockSourceWalker, MockMediaProbe, NoopEnricher> {
        LibraryScanHandler::new(repo, Scanner::new(walker, probe), NoopEnricher)
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
        assert!(state.started_at.is_some());
        assert!(state.last_scanned_at.is_some());
    }

    #[tokio::test]
    async fn a_long_scan_moves_the_progress_bar_while_it_runs() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library(&["/m"]));
        let walker = MockSourceWalker::new().with_entries(
            "/m",
            (0..500)
                .map(|n| WalkedEntry {
                    path: format!("/m/movie{n}.mkv"),
                    size_bytes: 1,
                })
                .collect(),
        );
        let handler = handler(repo.clone(), walker, MockMediaProbe::new());

        handler.handle(&scan_job("lib")).await.unwrap();

        assert_eq!(
            repo.recorded_scan_progress(),
            vec![0.0, 0.4, 0.8, 1.0],
            "an admin watching a long scan saw zero until it finished, because \
             nothing wrote a state between the running and idle rows"
        );
    }

    #[tokio::test]
    async fn enricher_runs_after_successful_scan() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library(&["/m"]));
        let called = Arc::new(AtomicBool::new(false));
        let handler = LibraryScanHandler::new(
            repo.clone(),
            Scanner::new(MockSourceWalker::new(), MockMediaProbe::new()),
            SpyEnricher {
                called: Arc::clone(&called),
            },
        );

        handler.handle(&scan_job("lib")).await.unwrap();

        assert!(called.load(Ordering::Relaxed));
        let state = repo
            .scan_state(&LibraryId("lib".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(state.status, ScanStatus::Idle);
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
        repo.save_scan_state(running(&LibraryId("lib".into()), Timestamp::now()))
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
    async fn triggered_scan_is_completed_by_the_worker() {
        use domain::repository::JobRepository;
        use domain::service::LibraryService;
        use domain::user::{Principal, Role, UserId};
        use mocks::{MockCatalogRepo, MockJobStore, MockMetadataProvider, MockUserRepo};
        use services::library::LibraryServiceImpl;

        let repo = MockLibraryRepo::new();
        repo.insert_library(library(&["/m"]));
        let jobs = MockJobStore::new();
        let svc = LibraryServiceImpl::new(
            repo.clone(),
            MockUserRepo::new(),
            jobs.clone(),
            MockCatalogRepo::new(),
            None::<MockMetadataProvider>,
        );
        let admin = Principal {
            user: UserId("admin".into()),
            role: Role::Admin,
        };
        let id = LibraryId("lib".into());

        svc.trigger_scan(&admin, &id).await.unwrap();
        let job = jobs.list().await.unwrap().into_iter().next().unwrap();

        let walker = MockSourceWalker::new().with_entries(
            "/m",
            vec![WalkedEntry {
                path: "/m/a.mkv".into(),
                size_bytes: 1,
            }],
        );
        let handler = handler(repo.clone(), walker, MockMediaProbe::new());
        handler.handle(&job).await.unwrap();

        let state = repo.scan_state(&id).await.unwrap().unwrap();
        assert_eq!(state.status, ScanStatus::Idle);
        assert!(state.last_scanned_at.is_some());
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

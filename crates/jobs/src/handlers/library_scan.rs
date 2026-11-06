use domain::catalog::VersionReconcile;
use domain::error::{RepositoryError, WalkError};
use domain::job::Job;
use domain::library::{
    LibraryId, ScanMode, ScanReport, ScanState, ScanStatus, SkipReason, SourceWalker,
};
use domain::media::MediaProbe;
use domain::repository::LibraryRepository;
use jiff::Timestamp;
use services::library::{EnrichSummary, ScanEnricher, ScanJobPayload, Scanner};

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct LibraryScanHandler<R, W, P, E> {
    repo: R,
    scanner: Scanner<W, P>,
    enricher: E,
}

impl<R, W, P, E> LibraryScanHandler<R, W, P, E> {
    pub fn new(repo: R, scanner: Scanner<W, P>, enricher: E) -> Self {
        Self { repo, scanner, enricher }
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
            return Err(JobError::Permanent("library scan job has empty payload".to_owned()));
        }
        let ScanJobPayload { library: id, mode } = ScanJobPayload::decode(&job.payload);
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
        self.repo.save_scan_state(running(&id, started)).await.map_err(retryable)?;

        let walk = match self.scanner.walk(&library).await {
            Ok(walk) => walk,
            Err(err) => {
                self.repo.save_scan_state(failed(&id, started, &err)).await.map_err(retryable)?;
                let message = err.to_string();
                return Err(match err {
                    WalkError::RootNotFound(_) => JobError::Permanent(message),
                    WalkError::Unreadable(_) => JobError::Retryable(message),
                });
            }
        };
        for path in &walk.offline {
            #[rustfmt::skip]
            tracing::warn!("library [{}] is offline at [{path}], its versions are left as they were", id.0);
        }
        let note = offline_note(&walk.offline);
        let present = walk.present();
        let versions = self.enricher.reconcile(&library, &present, &walk.offline).await;

        let repo = &self.repo;
        let report = self
            .scanner
            .probe_files(&library, walk, |done, total| {
                let state = probing(&id, started, done, total);
                async move {
                    if let Err(err) = repo.save_scan_state(state).await {
                        tracing::warn!("could not report scan progress: {err}");
                    }
                }
            })
            .await;
        if mode == ScanMode::Reread {
            self.enricher.reread(&library, &report).await;
        }
        let enriched = self.enricher.enrich(&library, &report, Some(&job.id)).await;
        let finished = Timestamp::now();
        self.repo.save_scan_state(idle(&id, started, finished, note)).await.map_err(retryable)?;
        let verb = if mode == ScanMode::Reread { "re-read" } else { "scanned" };
        let seconds = finished.duration_since(started).as_secs_f64();
        let summary = scan_summary(present.len(), &report, versions.as_ref(), enriched);
        #[rustfmt::skip]
        tracing::info!("library [{}] [{}] {verb} in [{seconds:.1} s]: {summary}", library.name, id.0);
        Ok(())
    }
}

fn scan_summary(
    found: usize,
    report: &ScanReport,
    versions: Option<&VersionReconcile>,
    enriched: EnrichSummary,
) -> String {
    let unreadable = report
        .skipped
        .iter()
        .filter(|file| matches!(file.reason, SkipReason::ProbeFailed(_)))
        .count();
    let excluded = report.skipped.len() - unreadable;
    let probed = report.discovered.len();
    let versions = match versions {
        Some(versions) => {
            let known = versions.known.len();
            let new = report
                .discovered
                .iter()
                .filter(|file| {
                    !versions.known.contains(&file.path) && !enriched.unmatched.contains(&file.path)
                })
                .count();
            let VersionReconcile { gone, returned, .. } = versions;
            format!("[{known}] known, [{new}] new, [{gone}] gone, [{returned}] back")
        }
        None => "versions not reconciled".to_owned(),
    };
    let EnrichSummary { matched, titles, unmatched, duplicates, failed } = enriched;
    let unmatched = unmatched.len();
    #[rustfmt::skip]
    let summary = format!("files [{found}] found, [{probed}] probed, [{unreadable}] unreadable, [{excluded}] excluded; {versions}; [{matched}] matched to [{titles}] titles, [{unmatched}] unmatched, [{duplicates}] duplicates, [{failed}] failed");
    summary
}

const NOTED_OFFLINE: usize = 5;

fn offline_note(offline: &[String]) -> Option<String> {
    if offline.is_empty() {
        return None;
    }
    let listed = offline[..offline.len().min(NOTED_OFFLINE)].join(", ");
    Some(match offline.len().saturating_sub(NOTED_OFFLINE) {
        0 => format!("Offline, versions left as they were: {listed}"),
        more => format!("Offline, versions left as they were: {listed} and {more} more"),
    })
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

fn idle(id: &LibraryId, started: Timestamp, now: Timestamp, note: Option<String>) -> ScanState {
    ScanState {
        library: id.clone(),
        status: ScanStatus::Idle,
        progress: 1.0,
        started_at: Some(started),
        last_scanned_at: Some(now),
        error: note,
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
    use std::sync::{Arc, Mutex};

    use super::*;
    use domain::error::ProbeError;
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use domain::library::{
        Library, LibraryKind, LibraryOrigin, ScanReport, WalkedEntry, WatcherStrategy,
    };
    use domain::media::ProbeResult;
    use mocks::{MockLibraryRepo, MockMediaProbe, MockSourceWalker};
    use services::library::NoopEnricher;
    use tracing_test::traced_test;

    type Reconciled = Arc<Mutex<Vec<(Vec<String>, Vec<String>)>>>;

    #[derive(Clone)]
    struct SpyEnricher {
        calls: Arc<Mutex<Vec<&'static str>>>,
        reconciled: Reconciled,
    }

    impl ScanEnricher for SpyEnricher {
        async fn reconcile(
            &self,
            _library: &Library,
            present: &[String],
            offline: &[String],
        ) -> Option<VersionReconcile> {
            self.calls.lock().unwrap().push("reconcile");
            self.reconciled.lock().unwrap().push((present.to_vec(), offline.to_vec()));
            None
        }

        async fn enrich(
            &self,
            _library: &Library,
            _report: &ScanReport,
            _parent: Option<&JobId>,
        ) -> EnrichSummary {
            self.calls.lock().unwrap().push("enrich");
            EnrichSummary::default()
        }

        async fn reread(&self, _library: &Library, _report: &ScanReport) {
            self.calls.lock().unwrap().push("reread");
        }
    }

    #[derive(Clone)]
    struct StatusSpy {
        repo: MockLibraryRepo,
        seen: Arc<Mutex<Vec<Option<ScanStatus>>>>,
    }

    impl StatusSpy {
        async fn record(&self, library: &Library) {
            let state = self.repo.scan_state(&library.id).await.unwrap();
            self.seen.lock().unwrap().push(state.map(|state| state.status));
        }
    }

    impl ScanEnricher for StatusSpy {
        async fn reconcile(
            &self,
            library: &Library,
            _present: &[String],
            _offline: &[String],
        ) -> Option<VersionReconcile> {
            self.record(library).await;
            None
        }

        async fn enrich(
            &self,
            library: &Library,
            _report: &ScanReport,
            _parent: Option<&JobId>,
        ) -> EnrichSummary {
            self.record(library).await;
            EnrichSummary::default()
        }

        async fn reread(&self, library: &Library, _report: &ScanReport) {
            self.record(library).await;
        }
    }

    fn reread_job() -> Job {
        scan_job(
            &ScanJobPayload { library: LibraryId("lib".into()), mode: ScanMode::Reread }.encode(),
        )
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
        let walker = MockSourceWalker::new()
            .with_entries("/m", vec![WalkedEntry { path: "/m/a.mkv".into(), size_bytes: 1 }]);
        let handler = handler(repo.clone(), walker, MockMediaProbe::new());

        handler.handle(&scan_job("lib")).await.unwrap();

        let state = repo.scan_state(&LibraryId("lib".into())).await.unwrap().unwrap();
        assert_eq!(state.status, ScanStatus::Idle);
        assert_eq!(state.progress, 1.0);
        assert!(state.started_at.is_some());
        assert!(state.last_scanned_at.is_some());
        assert_eq!(state.error, None, "a scan that walked every root leaves no note");
    }

    #[traced_test]
    #[tokio::test]
    async fn an_offline_root_is_logged_and_noted_on_the_finished_scan() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library(&["/m", "/unmounted"]));
        let walker = MockSourceWalker::new()
            .with_entries("/m", vec![WalkedEntry { path: "/m/a.mkv".into(), size_bytes: 1 }]);
        let handler = handler(repo.clone(), walker, MockMediaProbe::new());

        handler.handle(&scan_job("lib")).await.unwrap();

        let state = repo.scan_state(&LibraryId("lib".into())).await.unwrap().unwrap();
        assert_eq!(state.status, ScanStatus::Idle);
        assert_eq!(state.error.as_deref(), Some("Offline, versions left as they were: /unmounted"));
        assert!(logs_contain("library [lib] is offline at [/unmounted]"));
    }

    #[test]
    fn a_long_offline_list_is_cut_short_in_the_note() {
        let offline: Vec<String> = (1..=7).map(|n| format!("/m/{n}")).collect();

        assert_eq!(
            offline_note(&offline).as_deref(),
            Some("Offline, versions left as they were: /m/1, /m/2, /m/3, /m/4, /m/5 and 2 more")
        );
        assert_eq!(offline_note(&[]), None);
    }

    struct CountingEnricher;

    impl ScanEnricher for CountingEnricher {
        async fn reconcile(
            &self,
            _library: &Library,
            _present: &[String],
            _offline: &[String],
        ) -> Option<VersionReconcile> {
            Some(VersionReconcile {
                known: ["/m/good.mkv", "/m/bad.mkv", "/m/resolved.mkv"].map(str::to_owned).into(),
                gone: 2,
                returned: 1,
            })
        }

        async fn enrich(
            &self,
            _library: &Library,
            _report: &ScanReport,
            _parent: Option<&JobId>,
        ) -> EnrichSummary {
            EnrichSummary {
                matched: 2,
                titles: 2,
                unmatched: ["/m/recording.mkv", "/m/resolved.mkv"].map(str::to_owned).into(),
                duplicates: 4,
                failed: 5,
            }
        }

        async fn reread(&self, _library: &Library, _report: &ScanReport) {}
    }

    fn counted() -> impl JobHandler {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library(&["/m"]));
        let walker = MockSourceWalker::new().with_entries(
            "/m",
            [
                "/m/good.mkv",
                "/m/new.mkv",
                "/m/recording.mkv",
                "/m/resolved.mkv",
                "/m/bad.mkv",
                "/m/extras/trailer.mkv",
            ]
            .into_iter()
            .map(|path| WalkedEntry { path: path.into(), size_bytes: 1 })
            .collect(),
        );
        let probe = MockMediaProbe::new().failing_on("/m/bad.mkv");
        LibraryScanHandler::new(repo, Scanner::new(walker, probe), CountingEnricher)
    }

    #[traced_test]
    #[tokio::test]
    async fn a_finished_scan_logs_what_it_found_and_what_became_of_it() {
        counted().handle(&scan_job("lib")).await.unwrap();

        assert!(logs_contain("library [Lib] [lib] scanned in ["));
        assert!(
            logs_contain(
                "files [6] found, [4] probed, [1] unreadable, [1] excluded; [3] known, [1] new, \
                 [2] gone, [1] back; [2] matched to [2] titles, [2] unmatched, [4] duplicates, \
                 [5] failed"
            ),
            "an unmatched file never gets a version, so it is not new on every scan, and a \
             resolved one is known though its name still matches nothing"
        );
    }

    #[traced_test]
    #[tokio::test]
    async fn a_finished_reread_says_it_re_read() {
        counted().handle(&reread_job()).await.unwrap();

        assert!(logs_contain("library [Lib] [lib] re-read in ["));
    }

    #[test]
    fn a_failed_reconcile_leaves_the_version_counts_out() {
        let summary = scan_summary(0, &ScanReport::default(), None, EnrichSummary::default());

        assert_eq!(
            summary,
            "files [0] found, [0] probed, [0] unreadable, [0] excluded; versions not reconciled; \
             [0] matched to [0] titles, [0] unmatched, [0] duplicates, [0] failed"
        );
    }

    #[derive(Clone)]
    struct SpyProbe {
        calls: Arc<Mutex<Vec<&'static str>>>,
    }

    impl MediaProbe for SpyProbe {
        async fn probe(&self, path: &str) -> Result<ProbeResult, ProbeError> {
            self.calls.lock().unwrap().push("probe");
            MockMediaProbe::new().failing_on("/m/bad.mkv").probe(path).await
        }
    }

    #[tokio::test]
    async fn availability_is_saved_from_the_walk_before_the_first_probe() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library(&["/m"]));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let reconciled = Reconciled::default();
        let walker = MockSourceWalker::new().with_entries(
            "/m",
            vec![
                WalkedEntry { path: "/m/good.mkv".into(), size_bytes: 1 },
                WalkedEntry { path: "/m/bad.mkv".into(), size_bytes: 1 },
            ],
        );
        let handler = LibraryScanHandler::new(
            repo,
            Scanner::new(walker, SpyProbe { calls: Arc::clone(&calls) }),
            SpyEnricher { calls: Arc::clone(&calls), reconciled: Arc::clone(&reconciled) },
        );

        handler.handle(&scan_job("lib")).await.unwrap();

        assert_eq!(
            *calls.lock().unwrap(),
            ["reconcile", "probe", "probe", "enrich"],
            "versions came back only once every file was probed, so a large library stayed \
             unavailable long after its storage returned"
        );
        assert_eq!(
            *reconciled.lock().unwrap(),
            [(vec!["/m/good.mkv".to_owned(), "/m/bad.mkv".to_owned()], Vec::new())],
            "a file the probe cannot read still counts as on disk"
        );
    }

    #[tokio::test]
    async fn a_long_scan_moves_the_progress_bar_while_it_runs() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library(&["/m"]));
        let walker = MockSourceWalker::new().with_entries(
            "/m",
            (0..500)
                .map(|n| WalkedEntry { path: format!("/m/movie{n}.mkv"), size_bytes: 1 })
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

    fn spied(repo: &MockLibraryRepo) -> (Arc<Mutex<Vec<&'static str>>>, impl JobHandler) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let handler = LibraryScanHandler::new(
            repo.clone(),
            Scanner::new(MockSourceWalker::new(), MockMediaProbe::new()),
            SpyEnricher { calls: Arc::clone(&calls), reconciled: Reconciled::default() },
        );
        (calls, handler)
    }

    #[tokio::test]
    async fn enricher_runs_after_successful_scan() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library(&["/m"]));
        let (calls, handler) = spied(&repo);

        handler.handle(&scan_job("lib")).await.unwrap();

        assert_eq!(*calls.lock().unwrap(), vec!["reconcile", "enrich"]);
        let state = repo.scan_state(&LibraryId("lib".into())).await.unwrap().unwrap();
        assert_eq!(state.status, ScanStatus::Idle);
    }

    #[tokio::test]
    async fn a_reread_job_rereads_known_files_before_enriching() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library(&["/m"]));
        let (calls, handler) = spied(&repo);

        handler.handle(&reread_job()).await.unwrap();

        assert_eq!(*calls.lock().unwrap(), vec!["reconcile", "reread", "enrich"]);
    }

    #[tokio::test]
    async fn the_library_still_reads_as_scanning_while_enrichment_writes() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library(&["/m"]));
        let seen = Arc::new(Mutex::new(Vec::new()));
        let handler = LibraryScanHandler::new(
            repo.clone(),
            Scanner::new(MockSourceWalker::new(), MockMediaProbe::new()),
            StatusSpy { repo: repo.clone(), seen: Arc::clone(&seen) },
        );

        handler.handle(&reread_job()).await.unwrap();

        assert_eq!(
            *seen.lock().unwrap(),
            vec![Some(ScanStatus::Running); 3],
            "availability, re-reading and enrichment all write, so the scan is not over until \
             they return"
        );
        let state = repo.scan_state(&LibraryId("lib".into())).await.unwrap().unwrap();
        assert_eq!(state.status, ScanStatus::Idle, "and it is only idle once that is done");
    }

    #[tokio::test]
    async fn empty_payload_is_permanent() {
        let handler =
            handler(MockLibraryRepo::new(), MockSourceWalker::new(), MockMediaProbe::new());
        assert!(matches!(handler.handle(&scan_job("")).await.unwrap_err(), JobError::Permanent(_)));
    }

    #[tokio::test]
    async fn missing_library_is_permanent() {
        let handler =
            handler(MockLibraryRepo::new(), MockSourceWalker::new(), MockMediaProbe::new());
        assert!(matches!(
            handler.handle(&scan_job("nope")).await.unwrap_err(),
            JobError::Permanent(_)
        ));
    }

    #[tokio::test]
    async fn already_running_is_a_benign_skip() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library(&["/m"]));
        repo.save_scan_state(running(&LibraryId("lib".into()), Timestamp::now())).await.unwrap();
        let handler = handler(repo.clone(), MockSourceWalker::new(), MockMediaProbe::new());

        handler.handle(&scan_job("lib")).await.unwrap();

        let state = repo.scan_state(&LibraryId("lib".into())).await.unwrap().unwrap();
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
            account_admin: true,
            device: None,
            age_limit: None,
        };
        let id = LibraryId("lib".into());

        svc.trigger_scan(&admin, &id, ScanMode::Normal).await.unwrap();
        let job = jobs.list().await.unwrap().into_iter().next().unwrap();

        let walker = MockSourceWalker::new()
            .with_entries("/m", vec![WalkedEntry { path: "/m/a.mkv".into(), size_bytes: 1 }]);
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
        let state = repo.scan_state(&LibraryId("lib".into())).await.unwrap().unwrap();
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
        let state = repo.scan_state(&LibraryId("lib".into())).await.unwrap().unwrap();
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

    // Progress is a courtesy to whoever is watching, so a row that cannot be written must
    // not take the scan itself down with it.
    #[traced_test]
    #[tokio::test]
    async fn a_progress_row_that_cannot_be_written_does_not_fail_the_scan() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library(&["/m"]));
        let walker = MockSourceWalker::new().with_entries(
            "/m",
            (0..500)
                .map(|n| WalkedEntry { path: format!("/m/movie{n}.mkv"), size_bytes: 1 })
                .collect(),
        );
        let handler = handler(repo.clone(), walker, MockMediaProbe::new());
        repo.fail_saves_after(1);

        let result = handler.handle(&scan_job("lib")).await;

        assert!(result.is_err(), "the final state still has to be written");
        assert!(logs_contain("could not report scan progress"));
    }
}

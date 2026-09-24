use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use domain::error::RepositoryError;
use domain::job::{Job, JobKind, JobLogLevel, JobLogStore, JobStatus};
use domain::repository::JobRepository;
use jiff::Timestamp;
use tokio::sync::{Semaphore, watch};
use tokio::task::JoinSet;
use tokio::time::{Interval, interval};
use tracing::Instrument;

use crate::cancel::CancelRegistry;
use crate::error::JobError;
use crate::job_handler::JobHandler;
use crate::metrics::ActiveJob;
use crate::retry::{RetryPolicy, apply_outcome};

enum Outcome {
    Done(Result<(), JobError>),
    Cancelled,
}

struct Filled {
    claimed: usize,
    free: usize,
}

impl Filled {
    fn is_full(&self) -> bool {
        self.free > 0 && self.claimed == self.free
    }
}

async fn drive<'a>(
    mut work: Pin<Box<dyn Future<Output = Result<(), JobError>> + Send + 'a>>,
    cancelled: &mut watch::Receiver<bool>,
) -> Outcome {
    tokio::select! {
        biased;
        _ = cancelled.changed() => Outcome::Cancelled,
        result = &mut work => Outcome::Done(result),
    }
}

async fn finish_job<L: JobLogStore>(
    log: &L,
    job: &Job,
    active: ActiveJob,
    started: std::time::Instant,
    now: Timestamp,
    outcome: &Outcome,
) {
    let elapsed = started.elapsed().as_secs_f64();
    match outcome {
        Outcome::Done(result) => {
            active.finish(result.is_ok());
            let (level, message) = match result {
                Ok(()) => (
                    JobLogLevel::Info,
                    format!("finished {:?}: succeeded in {elapsed:.1}s", job.kind),
                ),
                Err(JobError::Retryable(message)) => (
                    JobLogLevel::Warn,
                    format!(
                        "finished {:?}: failed in {elapsed:.1}s (attempt {}), will retry: {message}",
                        job.kind,
                        job.attempts + 1
                    ),
                ),
                Err(JobError::Permanent(message)) => (
                    JobLogLevel::Error,
                    format!(
                        "finished {:?}: failed permanently in {elapsed:.1}s: {message}",
                        job.kind
                    ),
                ),
            };
            let _ = log.append(&job.id, now, level, &message).await;
        }
        Outcome::Cancelled => {
            active.finish(false);
            let _ = log
                .append(
                    &job.id,
                    now,
                    JobLogLevel::Warn,
                    &format!("cancelled {:?} after {elapsed:.1}s", job.kind),
                )
                .await;
        }
    }
}

async fn next_batch(ticker: &mut Interval, backlog: bool) {
    if backlog {
        tokio::task::yield_now().await;
    } else {
        ticker.tick().await;
    }
}

pub struct Worker<R, H, L> {
    repo: R,
    handler: Arc<H>,
    log: Arc<L>,
    limit: usize,
    policy: RetryPolicy,
    kinds: Vec<JobKind>,
    cancel: CancelRegistry,
    pool: Arc<str>,
}

impl<R, H, L> Worker<R, H, L>
where
    R: JobRepository,
    H: JobHandler + Send + Sync + 'static,
    L: JobLogStore + 'static,
{
    pub fn new(
        repo: R,
        handler: Arc<H>,
        log: L,
        limit: usize,
        policy: RetryPolicy,
        kinds: Vec<JobKind>,
    ) -> Self {
        Self {
            repo,
            handler,
            log: Arc::new(log),
            limit,
            policy,
            kinds,
            cancel: CancelRegistry::default(),
            pool: crate::job_pools::DEFAULT_POOL.into(),
        }
    }

    pub fn with_cancel(mut self, cancel: CancelRegistry) -> Self {
        self.cancel = cancel;
        self
    }

    pub fn with_pool(mut self, pool: impl Into<Arc<str>>) -> Self {
        self.pool = pool.into();
        self
    }

    async fn fill(
        &self,
        tasks: &mut JoinSet<(Job, Outcome)>,
        semaphore: &Arc<Semaphore>,
        now: Timestamp,
    ) -> Result<Filled, RepositoryError> {
        let free = semaphore.available_permits();
        if free == 0 {
            return Ok(Filled { claimed: 0, free });
        }
        let jobs = self.repo.claim_ready(now, free, self.kinds.clone()).await?;
        let claimed = jobs.len();
        for job in jobs {
            let permit = Arc::clone(semaphore)
                .acquire_owned()
                .await
                .expect("worker semaphore is never closed");
            let handler = Arc::clone(&self.handler);
            let log = Arc::clone(&self.log);
            let mut cancelled = self.cancel.register(&job.id);
            let pool = Arc::clone(&self.pool);
            tasks.spawn(async move {
                let _permit = permit;
                let span =
                    tracing::info_span!("job", job_id = %job.id.0, kind = ?job.kind, pool = %pool);
                let _ = log
                    .append(&job.id, now, JobLogLevel::Info, &format!("started {:?}", job.kind))
                    .await;
                let started = std::time::Instant::now();
                let active = ActiveJob::start(job.kind, &pool);
                let outcome =
                    drive(Box::pin(handler.handle(&job).instrument(span)), &mut cancelled).await;
                finish_job(&*log, &job, active, started, now, &outcome).await;
                (job, outcome)
            });
        }
        Ok(Filled { claimed, free })
    }

    async fn settle(
        &self,
        job: Job,
        outcome: Outcome,
        now: Timestamp,
    ) -> Result<(), RepositoryError> {
        self.cancel.deregister(&job.id);
        match outcome {
            Outcome::Done(result) => {
                self.repo.update(apply_outcome(job, result, now, &self.policy)).await?;
            }
            Outcome::Cancelled => {
                let mut job = job;
                job.status = JobStatus::Cancelled;
                job.finished_at = Some(now);
                job.updated_at = now;
                self.repo.update(job).await?;
            }
        }
        Ok(())
    }

    pub async fn run_once(&self, now: Timestamp) -> Result<usize, RepositoryError> {
        let semaphore = Arc::new(Semaphore::new(self.limit.max(1)));
        let mut tasks: JoinSet<(Job, Outcome)> = JoinSet::new();
        self.fill(&mut tasks, &semaphore, now).await?;
        let mut processed = 0;
        while let Some(joined) = tasks.join_next().await {
            let (job, outcome) = joined.expect("worker job task panicked");
            self.settle(job, outcome, now).await?;
            processed += 1;
        }
        Ok(processed)
    }

    pub async fn run(
        &self,
        period: Duration,
        shutdown: impl Future<Output = ()>,
    ) -> Result<(), RepositoryError> {
        let mut ticker = interval(period);
        tokio::pin!(shutdown);
        #[rustfmt::skip]
        tracing::info!("job pool [{}] running [{}] job(s) at a time over [{}] kind(s)", self.pool, self.limit, self.kinds.len());
        let semaphore = Arc::new(Semaphore::new(self.limit.max(1)));
        let mut tasks: JoinSet<(Job, Outcome)> = JoinSet::new();
        let mut backlog = false;
        loop {
            tokio::select! {
                biased;
                () = &mut shutdown => break,
                Some(joined) = tasks.join_next(), if !tasks.is_empty() => {
                    let (job, outcome) = joined.expect("worker job task panicked");
                    self.settle(job, outcome, Timestamp::now()).await?;
                    backlog = true;
                }
                () = next_batch(&mut ticker, backlog) => {
                    let filled = self.fill(&mut tasks, &semaphore, Timestamp::now()).await?;
                    backlog = filled.is_full();
                }
            }
        }
        while let Some(joined) = tasks.join_next().await {
            let (job, outcome) = joined.expect("worker job task panicked");
            self.settle(job, outcome, Timestamp::now()).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use mocks::{MockJobLogStore, MockJobStore};
    use tokio::sync::oneshot;

    struct OkHandler;
    impl JobHandler for OkHandler {
        async fn handle(&self, _job: &Job) -> Result<(), JobError> {
            Ok(())
        }
    }

    struct RetryHandler;
    impl JobHandler for RetryHandler {
        async fn handle(&self, _job: &Job) -> Result<(), JobError> {
            Err(JobError::Retryable("boom".into()))
        }
    }

    struct PermanentHandler;
    impl JobHandler for PermanentHandler {
        async fn handle(&self, _job: &Job) -> Result<(), JobError> {
            Err(JobError::Permanent("nope".into()))
        }
    }

    struct SlowHandler;
    impl JobHandler for SlowHandler {
        async fn handle(&self, _job: &Job) -> Result<(), JobError> {
            tokio::time::sleep(Duration::from_secs(1)).await;
            Ok(())
        }
    }

    fn test_kinds() -> Vec<JobKind> {
        JobKind::ALL.to_vec()
    }

    fn job(id: &str, priority: JobPriority, now: Timestamp) -> Job {
        Job {
            id: JobId(id.into()),
            kind: JobKind::LibraryScan,
            status: JobStatus::Queued,
            priority,
            payload: String::new(),
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

    #[tokio::test]
    async fn run_once_processes_success() {
        let now = Timestamp::now();
        let store = MockJobStore::new();
        store.enqueue(job("a", JobPriority::Normal, now)).await.unwrap();
        let log = MockJobLogStore::new();
        let worker = Worker::new(
            store.clone(),
            Arc::new(OkHandler),
            log.clone(),
            4,
            RetryPolicy::default(),
            test_kinds(),
        );

        assert_eq!(worker.run_once(now).await.unwrap(), 1);
        let stored = store.get(&JobId("a".into())).await.unwrap().unwrap();
        assert_eq!(stored.status, JobStatus::Succeeded);

        let lines = log.lines_for(&JobId("a".into()));
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("INFO started LibraryScan"));
        assert!(lines[1].contains("INFO finished LibraryScan: succeeded in"));
    }

    #[tokio::test]
    async fn run_once_requeues_retryable() {
        let now = Timestamp::now();
        let store = MockJobStore::new();
        store.enqueue(job("a", JobPriority::Normal, now)).await.unwrap();
        let log = MockJobLogStore::new();
        let worker = Worker::new(
            store.clone(),
            Arc::new(RetryHandler),
            log.clone(),
            4,
            RetryPolicy::default(),
            test_kinds(),
        );

        worker.run_once(now).await.unwrap();
        let stored = store.get(&JobId("a".into())).await.unwrap().unwrap();
        assert_eq!(stored.status, JobStatus::Queued);
        assert_eq!(stored.attempts, 1);
        assert!(stored.available_at > now);
        assert!(stored.last_error.is_some());

        let lines = log.lines_for(&JobId("a".into()));
        assert!(lines[1].contains("WARN finished LibraryScan: failed in"));
        assert!(lines[1].contains("(attempt 1), will retry: boom"));
    }

    #[tokio::test]
    async fn run_once_fails_permanent() {
        let now = Timestamp::now();
        let store = MockJobStore::new();
        store.enqueue(job("a", JobPriority::Normal, now)).await.unwrap();
        let log = MockJobLogStore::new();
        let worker = Worker::new(
            store.clone(),
            Arc::new(PermanentHandler),
            log.clone(),
            4,
            RetryPolicy::default(),
            test_kinds(),
        );

        worker.run_once(now).await.unwrap();
        let stored = store.get(&JobId("a".into())).await.unwrap().unwrap();
        assert_eq!(stored.status, JobStatus::Failed);

        let lines = log.lines_for(&JobId("a".into()));
        assert!(lines[1].contains("ERROR finished LibraryScan: failed permanently in"));
        assert!(lines[1].contains(": nope"));
    }

    #[tokio::test]
    async fn run_once_honours_concurrency_limit() {
        let now = Timestamp::now();
        let store = MockJobStore::new();
        for i in 0..5 {
            store.enqueue(job(&format!("j{i}"), JobPriority::Normal, now)).await.unwrap();
        }
        let worker = Worker::new(
            store.clone(),
            Arc::new(OkHandler),
            MockJobLogStore::new(),
            2,
            RetryPolicy::default(),
            test_kinds(),
        );

        assert_eq!(worker.run_once(now).await.unwrap(), 2);
        let queued = store
            .list()
            .await
            .unwrap()
            .into_iter()
            .filter(|j| j.status == JobStatus::Queued)
            .count();
        assert_eq!(queued, 3);
    }

    #[tracing_test::traced_test]
    #[tokio::test]
    async fn a_worker_names_its_pool_when_it_starts() {
        let (tx, rx) = oneshot::channel::<()>();
        let worker = Worker::new(
            MockJobStore::new(),
            Arc::new(OkHandler),
            MockJobLogStore::new(),
            2,
            RetryPolicy::default(),
            vec![JobKind::Trickplay],
        )
        .with_pool("trickplay");

        tx.send(()).unwrap();
        worker.run(Duration::from_millis(5), async { rx.await.unwrap_or(()) }).await.unwrap();

        assert!(logs_contain("job pool [trickplay] running [2] job(s) at a time over [1] kind(s)"));
    }

    #[tokio::test]
    async fn worker_claims_only_its_own_kinds() {
        let now = Timestamp::now();
        let store = MockJobStore::new();
        store.enqueue(job("scan", JobPriority::Normal, now)).await.unwrap();
        let mut transcribe = job("transcribe", JobPriority::High, now);
        transcribe.kind = JobKind::Transcription;
        store.enqueue(transcribe).await.unwrap();
        let worker = Worker::new(
            store.clone(),
            Arc::new(OkHandler),
            MockJobLogStore::new(),
            4,
            RetryPolicy::default(),
            vec![JobKind::LibraryScan],
        );

        assert_eq!(worker.run_once(now).await.unwrap(), 1);
        assert_eq!(
            store.get(&JobId("scan".into())).await.unwrap().unwrap().status,
            JobStatus::Succeeded
        );
        assert_eq!(
            store.get(&JobId("transcribe".into())).await.unwrap().unwrap().status,
            JobStatus::Queued
        );
    }

    #[tokio::test(start_paused = true)]
    async fn run_drives_until_shutdown() {
        let now = Timestamp::now();
        let store = MockJobStore::new();
        store.enqueue(job("a", JobPriority::Normal, now)).await.unwrap();
        let worker = Worker::new(
            store.clone(),
            Arc::new(OkHandler),
            MockJobLogStore::new(),
            4,
            RetryPolicy::default(),
            test_kinds(),
        );

        let (tx, rx) = oneshot::channel::<()>();
        let driver = worker.run(Duration::from_millis(10), async move {
            let _ = rx.await;
        });
        let control = async {
            tokio::time::sleep(Duration::from_millis(5)).await;
            let _ = tx.send(());
        };
        let (result, ()) = tokio::join!(driver, control);
        result.unwrap();

        let stored = store.get(&JobId("a".into())).await.unwrap().unwrap();
        assert_eq!(stored.status, JobStatus::Succeeded);
    }

    #[tokio::test(start_paused = true)]
    async fn one_tick_drains_a_backlog_deeper_than_the_batch() {
        let now = Timestamp::now();
        let store = MockJobStore::new();
        for i in 0..50 {
            store.enqueue(job(&format!("j{i:02}"), JobPriority::Normal, now)).await.unwrap();
        }
        let worker = Worker::new(
            store.clone(),
            Arc::new(OkHandler),
            MockJobLogStore::new(),
            4,
            RetryPolicy::default(),
            test_kinds(),
        );

        worker
            .run(Duration::from_secs(3600), async {
                tokio::time::sleep(Duration::from_secs(1800)).await;
            })
            .await
            .unwrap();

        let done = store
            .list()
            .await
            .unwrap()
            .into_iter()
            .filter(|j| j.status == JobStatus::Succeeded)
            .count();
        assert_eq!(done, 50, "the first tick should have drained the queue");
    }

    #[tokio::test(start_paused = true)]
    async fn shutdown_still_wins_against_a_queue_that_never_empties() {
        let now = Timestamp::now();
        let store = MockJobStore::new();
        for i in 0..50 {
            store.enqueue(job(&format!("j{i:02}"), JobPriority::Normal, now)).await.unwrap();
        }
        let worker = Worker::new(
            store.clone(),
            Arc::new(SlowHandler),
            MockJobLogStore::new(),
            4,
            RetryPolicy::default(),
            test_kinds(),
        );

        worker
            .run(Duration::from_secs(3600), async {
                tokio::time::sleep(Duration::from_millis(2500)).await;
            })
            .await
            .unwrap();

        let done = store
            .list()
            .await
            .unwrap()
            .into_iter()
            .filter(|j| j.status == JobStatus::Succeeded)
            .count();
        assert!(
            (8..50).contains(&done),
            "draining should stop at the first batch boundary after shutdown, got {done}"
        );
    }

    fn recorded<F, Fut>(work: F) -> String
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let recorder = metrics_exporter_prometheus::PrometheusBuilder::new().build_recorder();
        let handle = recorder.handle();
        metrics::with_local_recorder(&recorder, || {
            let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
            rt.block_on(work());
        });
        handle.render()
    }

    #[test]
    fn records_success_metrics() {
        let rendered = recorded(|| async {
            let now = Timestamp::now();
            let store = MockJobStore::new();
            store.enqueue(job("a", JobPriority::Normal, now)).await.unwrap();
            let worker = Worker::new(
                store,
                Arc::new(OkHandler),
                MockJobLogStore::new(),
                4,
                RetryPolicy::default(),
                test_kinds(),
            )
            .with_pool("trickplay");
            worker.run_once(now).await.unwrap();
        });
        assert!(rendered.contains("jobs_total"));
        assert!(rendered.contains("job=\"library_scan\""));
        assert!(rendered.contains("outcome=\"succeeded\""));
        assert!(rendered.contains("job_duration_milliseconds"));
        assert!(rendered.contains("jobs_active{pool=\"trickplay\"} 0"));
    }

    #[test]
    fn records_failure_metrics() {
        let rendered = recorded(|| async {
            let now = Timestamp::now();
            let store = MockJobStore::new();
            store.enqueue(job("a", JobPriority::Normal, now)).await.unwrap();
            let worker = Worker::new(
                store,
                Arc::new(PermanentHandler),
                MockJobLogStore::new(),
                4,
                RetryPolicy::default(),
                test_kinds(),
            );
            worker.run_once(now).await.unwrap();
        });
        assert!(rendered.contains("outcome=\"failed\""));
        assert!(rendered.contains("jobs_active{pool=\"default\"} 0"));
    }

    #[tokio::test]
    async fn a_slow_job_no_longer_idles_the_rest_of_its_pool() {
        use tokio::sync::Notify;

        struct MixedHandler {
            hold: Arc<Notify>,
        }
        impl JobHandler for MixedHandler {
            async fn handle(&self, job: &Job) -> Result<(), JobError> {
                if job.id.0 == "a-slow" {
                    self.hold.notified().await;
                }
                Ok(())
            }
        }

        let now = Timestamp::now();
        let store = MockJobStore::new();
        for id in ["a-slow", "b-fast-1", "b-fast-2", "b-fast-3"] {
            store.enqueue(job(id, JobPriority::Normal, now)).await.unwrap();
        }
        let hold = Arc::new(Notify::new());
        let worker = Worker::new(
            store.clone(),
            Arc::new(MixedHandler { hold: hold.clone() }),
            MockJobLogStore::new(),
            2,
            RetryPolicy::default(),
            test_kinds(),
        );
        let (stop, stopped) = oneshot::channel::<()>();
        let run = tokio::spawn(async move {
            worker.run(Duration::from_millis(5), async { stopped.await.unwrap_or(()) }).await
        });

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let done = succeeded(&store, &["b-fast-1", "b-fast-2", "b-fast-3"]).await;
            let slow = store.get(&JobId("a-slow".into())).await.unwrap().unwrap();
            if done == 3 {
                #[rustfmt::skip]
                assert_eq!(slow.status, JobStatus::Running, "the slow job finished first, so this proves nothing");
                break;
            }
            #[rustfmt::skip]
            assert!(std::time::Instant::now() < deadline, "the pool stalled behind its slowest job: only [{done}] of 3 got through");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        hold.notify_waiters();
        stop.send(()).unwrap();
        run.await.unwrap().unwrap();
        assert_eq!(
            store.get(&JobId("a-slow".into())).await.unwrap().unwrap().status,
            JobStatus::Succeeded,
            "shutdown abandoned a running job"
        );
    }

    async fn succeeded(store: &MockJobStore, ids: &[&str]) -> usize {
        let mut done = 0;
        for id in ids {
            let job = store.get(&JobId((*id).into())).await.unwrap().unwrap();
            if job.status == JobStatus::Succeeded {
                done += 1;
            }
        }
        done
    }

    #[tokio::test]
    async fn run_once_cancels_running_job() {
        use std::sync::atomic::{AtomicBool, Ordering};

        use tokio::sync::Notify;

        use crate::cancel::CancelRegistry;

        struct DropFlag(Arc<AtomicBool>);
        impl Drop for DropFlag {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }

        struct ParkHandler {
            started: Arc<Notify>,
            dropped: Arc<AtomicBool>,
        }
        impl JobHandler for ParkHandler {
            async fn handle(&self, _job: &Job) -> Result<(), JobError> {
                let _flag = DropFlag(self.dropped.clone());
                self.started.notify_one();
                std::future::pending::<Result<(), JobError>>().await
            }
        }

        let now = Timestamp::now();
        let store = MockJobStore::new();
        store.enqueue(job("a", JobPriority::Normal, now)).await.unwrap();
        let registry = CancelRegistry::default();
        let started = Arc::new(Notify::new());
        let dropped = Arc::new(AtomicBool::new(false));
        let worker = Worker::new(
            store.clone(),
            Arc::new(ParkHandler { started: started.clone(), dropped: dropped.clone() }),
            MockJobLogStore::new(),
            4,
            RetryPolicy::default(),
            test_kinds(),
        )
        .with_cancel(registry.clone());

        let run = tokio::spawn(async move { worker.run_once(now).await });
        started.notified().await;
        assert!(registry.cancel(&JobId("a".into())));
        let processed = run.await.unwrap().unwrap();

        assert_eq!(processed, 1);
        assert!(dropped.load(Ordering::SeqCst), "handler future should be dropped on cancel");
        let stored = store.get(&JobId("a".into())).await.unwrap().unwrap();
        assert_eq!(stored.status, JobStatus::Cancelled);
        assert!(stored.finished_at.is_some());
    }
}

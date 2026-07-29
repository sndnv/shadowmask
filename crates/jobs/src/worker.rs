use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use domain::error::RepositoryError;
use domain::job::{Job, JobKind, JobLogLevel, JobLogStore};
use domain::repository::JobRepository;
use jiff::Timestamp;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time::interval;
use tracing::Instrument;

use crate::error::JobError;
use crate::job_handler::JobHandler;
use crate::metrics::ActiveJob;
use crate::retry::{RetryPolicy, apply_outcome};

pub struct Worker<R, H, L> {
    repo: R,
    handler: Arc<H>,
    log: Arc<L>,
    limit: usize,
    policy: RetryPolicy,
    kinds: Vec<JobKind>,
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
        }
    }

    pub async fn run_once(&self, now: Timestamp) -> Result<usize, RepositoryError> {
        let jobs = self
            .repo
            .claim_ready(now, self.limit, self.kinds.clone())
            .await?;
        let semaphore = Arc::new(Semaphore::new(self.limit.max(1)));
        let mut tasks: JoinSet<(Job, Result<(), JobError>)> = JoinSet::new();
        for job in jobs {
            let permit = Arc::clone(&semaphore)
                .acquire_owned()
                .await
                .expect("worker semaphore is never closed");
            let handler = Arc::clone(&self.handler);
            let log = Arc::clone(&self.log);
            tasks.spawn(async move {
                let _permit = permit;
                let span = tracing::info_span!("job", job_id = %job.id.0, kind = ?job.kind);
                let _ = log
                    .append(
                        &job.id,
                        now,
                        JobLogLevel::Info,
                        &format!("started {:?}", job.kind),
                    )
                    .await;
                let started = std::time::Instant::now();
                let active = ActiveJob::start(job.kind);
                let result = handler.handle(&job).instrument(span).await;
                active.finish(result.is_ok());
                let elapsed = started.elapsed().as_secs_f64();
                let (level, message) = match &result {
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
                (job, result)
            });
        }
        let mut processed = 0;
        while let Some(joined) = tasks.join_next().await {
            let (job, result) = joined.expect("worker job task panicked");
            self.repo
                .update(apply_outcome(job, result, now, &self.policy))
                .await?;
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
        loop {
            tokio::select! {
                biased;
                () = &mut shutdown => return Ok(()),
                _ = ticker.tick() => {
                    self.run_once(Timestamp::now()).await?;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use services::mock::{MockJobLogStore, MockJobStore};
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

    fn test_kinds() -> Vec<JobKind> {
        crate::job_class::ALL_KINDS.to_vec()
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
        store
            .enqueue(job("a", JobPriority::Normal, now))
            .await
            .unwrap();
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
        store
            .enqueue(job("a", JobPriority::Normal, now))
            .await
            .unwrap();
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
        store
            .enqueue(job("a", JobPriority::Normal, now))
            .await
            .unwrap();
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
            store
                .enqueue(job(&format!("j{i}"), JobPriority::Normal, now))
                .await
                .unwrap();
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

    #[tokio::test]
    async fn worker_claims_only_its_own_kinds() {
        let now = Timestamp::now();
        let store = MockJobStore::new();
        store
            .enqueue(job("scan", JobPriority::Normal, now))
            .await
            .unwrap();
        let mut transcribe = job("transcribe", JobPriority::High, now);
        transcribe.kind = JobKind::Transcription;
        store.enqueue(transcribe).await.unwrap();
        let worker = Worker::new(
            store.clone(),
            Arc::new(OkHandler),
            MockJobLogStore::new(),
            4,
            RetryPolicy::default(),
            crate::job_class::normal_kinds(),
        );

        assert_eq!(worker.run_once(now).await.unwrap(), 1);
        assert_eq!(
            store
                .get(&JobId("scan".into()))
                .await
                .unwrap()
                .unwrap()
                .status,
            JobStatus::Succeeded
        );
        assert_eq!(
            store
                .get(&JobId("transcribe".into()))
                .await
                .unwrap()
                .unwrap()
                .status,
            JobStatus::Queued
        );
    }

    #[tokio::test(start_paused = true)]
    async fn run_drives_until_shutdown() {
        let now = Timestamp::now();
        let store = MockJobStore::new();
        store
            .enqueue(job("a", JobPriority::Normal, now))
            .await
            .unwrap();
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

    fn recorded<F, Fut>(work: F) -> String
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let recorder = metrics_exporter_prometheus::PrometheusBuilder::new().build_recorder();
        let handle = recorder.handle();
        metrics::with_local_recorder(&recorder, || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap();
            rt.block_on(work());
        });
        handle.render()
    }

    #[test]
    fn records_success_metrics() {
        let rendered = recorded(|| async {
            let now = Timestamp::now();
            let store = MockJobStore::new();
            store
                .enqueue(job("a", JobPriority::Normal, now))
                .await
                .unwrap();
            let worker = Worker::new(
                store,
                Arc::new(OkHandler),
                MockJobLogStore::new(),
                4,
                RetryPolicy::default(),
                test_kinds(),
            );
            worker.run_once(now).await.unwrap();
        });
        assert!(rendered.contains("jobs_total"));
        assert!(rendered.contains("job=\"library_scan\""));
        assert!(rendered.contains("outcome=\"succeeded\""));
        assert!(rendered.contains("job_duration_milliseconds"));
        assert!(rendered.contains("jobs_active 0"));
    }

    #[test]
    fn records_failure_metrics() {
        let rendered = recorded(|| async {
            let now = Timestamp::now();
            let store = MockJobStore::new();
            store
                .enqueue(job("a", JobPriority::Normal, now))
                .await
                .unwrap();
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
        assert!(rendered.contains("jobs_active 0"));
    }
}

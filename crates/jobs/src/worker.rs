use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use domain::error::RepositoryError;
use domain::job::Job;
use domain::repository::JobRepository;
use jiff::Timestamp;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time::interval;

use crate::error::JobError;
use crate::job_handler::JobHandler;
use crate::metrics::ActiveJob;
use crate::retry::{RetryPolicy, apply_outcome};

pub struct Worker<R, H> {
    repo: R,
    handler: Arc<H>,
    limit: usize,
    policy: RetryPolicy,
}

impl<R, H> Worker<R, H>
where
    R: JobRepository,
    H: JobHandler + Send + Sync + 'static,
{
    pub fn new(repo: R, handler: H, limit: usize, policy: RetryPolicy) -> Self {
        Self {
            repo,
            handler: Arc::new(handler),
            limit,
            policy,
        }
    }

    pub async fn run_once(&self, now: Timestamp) -> Result<usize, RepositoryError> {
        let jobs = self.repo.claim_ready(now, self.limit).await?;
        let semaphore = Arc::new(Semaphore::new(self.limit.max(1)));
        let mut tasks: JoinSet<(Job, Result<(), JobError>)> = JoinSet::new();
        for job in jobs {
            let permit = Arc::clone(&semaphore)
                .acquire_owned()
                .await
                .expect("worker semaphore is never closed");
            let handler = Arc::clone(&self.handler);
            tasks.spawn(async move {
                let _permit = permit;
                let active = ActiveJob::start(job.kind);
                let result = handler.handle(&job).await;
                active.finish(result.is_ok());
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
    use services::mock::MockJobStore;
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
        let worker = Worker::new(store.clone(), OkHandler, 4, RetryPolicy::default());

        assert_eq!(worker.run_once(now).await.unwrap(), 1);
        let stored = store.get(&JobId("a".into())).await.unwrap().unwrap();
        assert_eq!(stored.status, JobStatus::Succeeded);
    }

    #[tokio::test]
    async fn run_once_requeues_retryable() {
        let now = Timestamp::now();
        let store = MockJobStore::new();
        store
            .enqueue(job("a", JobPriority::Normal, now))
            .await
            .unwrap();
        let worker = Worker::new(store.clone(), RetryHandler, 4, RetryPolicy::default());

        worker.run_once(now).await.unwrap();
        let stored = store.get(&JobId("a".into())).await.unwrap().unwrap();
        assert_eq!(stored.status, JobStatus::Queued);
        assert_eq!(stored.attempts, 1);
        assert!(stored.available_at > now);
        assert!(stored.last_error.is_some());
    }

    #[tokio::test]
    async fn run_once_fails_permanent() {
        let now = Timestamp::now();
        let store = MockJobStore::new();
        store
            .enqueue(job("a", JobPriority::Normal, now))
            .await
            .unwrap();
        let worker = Worker::new(store.clone(), PermanentHandler, 4, RetryPolicy::default());

        worker.run_once(now).await.unwrap();
        let stored = store.get(&JobId("a".into())).await.unwrap().unwrap();
        assert_eq!(stored.status, JobStatus::Failed);
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
        let worker = Worker::new(store.clone(), OkHandler, 2, RetryPolicy::default());

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

    #[tokio::test(start_paused = true)]
    async fn run_drives_until_shutdown() {
        let now = Timestamp::now();
        let store = MockJobStore::new();
        store
            .enqueue(job("a", JobPriority::Normal, now))
            .await
            .unwrap();
        let worker = Worker::new(store.clone(), OkHandler, 4, RetryPolicy::default());

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
            let worker = Worker::new(store, OkHandler, 4, RetryPolicy::default());
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
            let worker = Worker::new(store, PermanentHandler, 4, RetryPolicy::default());
            worker.run_once(now).await.unwrap();
        });
        assert!(rendered.contains("outcome=\"failed\""));
        assert!(rendered.contains("jobs_active 0"));
    }
}

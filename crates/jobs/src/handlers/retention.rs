use domain::job::{Job, JobLogStore};
use domain::repository::JobRepository;
use jiff::{SignedDuration, Timestamp};

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct RetentionHandler<J, L> {
    jobs: J,
    logs: L,
    retain_for: SignedDuration,
}

impl<J, L> RetentionHandler<J, L> {
    pub fn new(jobs: J, logs: L, retain_for: SignedDuration) -> Self {
        Self { jobs, logs, retain_for }
    }
}

impl<J, L> JobHandler for RetentionHandler<J, L>
where
    J: JobRepository + Send + Sync,
    L: JobLogStore,
{
    async fn handle(&self, _job: &Job) -> Result<(), JobError> {
        let now = Timestamp::now();
        let Some(cutoff) = now.checked_sub(self.retain_for).ok() else {
            return Err(JobError::Permanent("retention window is out of range".into()));
        };
        let removed = self
            .jobs
            .delete_finished_before(cutoff)
            .await
            .map_err(|err| JobError::Retryable(err.to_string()))?;
        for id in &removed {
            if let Err(err) = self.logs.wipe(id).await {
                tracing::warn!(job = %id.0, "removing the log of an expired job failed: {err}");
            }
        }
        tracing::info!(removed = removed.len(), "job retention complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use domain::error::JobLogError;
    use domain::job::{JobId, JobKind, JobLogLevel, JobPriority, JobStatus};
    use mocks::{MockJobLogStore, MockJobStore};

    use super::*;

    fn retention_job() -> Job {
        job("retention", JobStatus::Running, None)
    }

    fn job(id: &str, status: JobStatus, finished_at: Option<Timestamp>) -> Job {
        let now = Timestamp::UNIX_EPOCH;
        Job {
            id: JobId(id.into()),
            kind: JobKind::Retention,
            status,
            priority: JobPriority::Normal,
            payload: String::new(),
            attempts: 0,
            progress: 0.0,
            available_at: now,
            last_error: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            finished_at,
            parent_id: None,
        }
    }

    async fn seed(store: &MockJobStore, id: &str, status: JobStatus, age: SignedDuration) {
        let finished_at =
            if status.is_active() { None } else { Timestamp::now().checked_sub(age).ok() };
        store.enqueue(job(id, status, finished_at)).await.unwrap();
    }

    #[tokio::test]
    async fn expired_terminal_jobs_are_removed_with_their_logs() {
        let jobs = MockJobStore::new();
        let logs = MockJobLogStore::new();
        seed(&jobs, "old", JobStatus::Succeeded, SignedDuration::from_hours(48)).await;
        logs.append(&JobId("old".into()), Timestamp::UNIX_EPOCH, JobLogLevel::Info, "a line")
            .await
            .unwrap();

        RetentionHandler::new(jobs.clone(), logs.clone(), SignedDuration::from_hours(24))
            .handle(&retention_job())
            .await
            .unwrap();

        assert!(jobs.get(&JobId("old".into())).await.unwrap().is_none());
        assert!(logs.lines_for(&JobId("old".into())).is_empty());
    }

    #[tokio::test]
    async fn recent_and_active_jobs_are_kept() {
        let jobs = MockJobStore::new();
        let logs = MockJobLogStore::new();
        seed(&jobs, "recent", JobStatus::Failed, SignedDuration::from_hours(1)).await;
        seed(&jobs, "queued", JobStatus::Queued, SignedDuration::from_hours(0)).await;
        seed(&jobs, "running", JobStatus::Running, SignedDuration::from_hours(0)).await;

        RetentionHandler::new(jobs.clone(), logs, SignedDuration::from_hours(24))
            .handle(&retention_job())
            .await
            .unwrap();

        for id in ["recent", "queued", "running"] {
            assert!(jobs.get(&JobId(id.into())).await.unwrap().is_some());
        }
    }

    #[tokio::test]
    async fn a_retention_window_that_cannot_be_subtracted_is_permanent() {
        let window = SignedDuration::MAX;
        let error = RetentionHandler::new(MockJobStore::new(), MockJobLogStore::new(), window)
            .handle(&retention_job())
            .await
            .unwrap_err();
        assert!(matches!(error, JobError::Permanent(_)));
    }

    #[tokio::test]
    async fn a_job_is_still_dropped_when_wiping_its_log_fails() {
        let jobs = MockJobStore::new();
        let logs = FailingWipe(MockJobLogStore::new());
        seed(&jobs, "old", JobStatus::Succeeded, SignedDuration::from_hours(48)).await;
        logs.append(&JobId("old".into()), Timestamp::UNIX_EPOCH, JobLogLevel::Info, "a line")
            .await
            .unwrap();

        RetentionHandler::new(jobs.clone(), logs.clone(), SignedDuration::from_hours(24))
            .handle(&retention_job())
            .await
            .unwrap();

        assert!(jobs.get(&JobId("old".into())).await.unwrap().is_none());
        assert_eq!(
            logs.read(&JobId("old".into()), None).await.unwrap().len(),
            1,
            "the log line outlives the job when the wipe fails"
        );
    }

    #[derive(Clone)]
    struct FailingWipe(MockJobLogStore);

    impl JobLogStore for FailingWipe {
        async fn append(
            &self,
            job: &JobId,
            at: Timestamp,
            level: JobLogLevel,
            message: &str,
        ) -> Result<(), JobLogError> {
            self.0.append(job, at, level, message).await
        }

        async fn read(&self, job: &JobId, tail: Option<usize>) -> Result<Vec<String>, JobLogError> {
            self.0.read(job, tail).await
        }

        async fn wipe(&self, _job: &JobId) -> Result<(), JobLogError> {
            Err(JobLogError::Backend("disk is on fire".into()))
        }
    }
}

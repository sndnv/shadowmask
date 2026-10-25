use domain::error::RepositoryError;
use domain::job::{Job, JobLogStore};
use domain::repository::{JobRepository, ReportRepository};
use jiff::{SignedDuration, Timestamp};

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct RetentionHandler<J, L, R> {
    jobs: J,
    logs: L,
    reports: R,
    retain_for: SignedDuration,
    reports_for: SignedDuration,
}

impl<J, L, R> RetentionHandler<J, L, R> {
    pub fn new(
        jobs: J,
        logs: L,
        reports: R,
        retain_for: SignedDuration,
        reports_for: SignedDuration,
    ) -> Self {
        Self { jobs, logs, reports, retain_for, reports_for }
    }
}

fn cutoff(now: Timestamp, window: SignedDuration) -> Timestamp {
    now.checked_sub(window).unwrap_or(Timestamp::MIN)
}

fn retryable(err: RepositoryError) -> JobError {
    JobError::Retryable(err.to_string())
}

impl<J, L, R> JobHandler for RetentionHandler<J, L, R>
where
    J: JobRepository + Send + Sync,
    L: JobLogStore,
    R: ReportRepository + Send + Sync,
{
    async fn handle(&self, _job: &Job) -> Result<(), JobError> {
        let now = Timestamp::now();
        let jobs_before = cutoff(now, self.retain_for);
        let reports_before = cutoff(now, self.reports_for);
        let removed = self.jobs.delete_finished_before(jobs_before).await.map_err(retryable)?;
        for id in &removed {
            if let Err(err) = self.logs.wipe(id).await {
                tracing::warn!(job = %id.0, "removing the log of an expired job failed: {err}");
            }
        }
        let reports = self.reports.delete_before(reports_before).await.map_err(retryable)?;
        tracing::info!(removed = removed.len(), reports, "job retention complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use domain::common::{LogLevel, PageRequest};
    use domain::diagnostics::{PlayerId, Report, ReportId, ReportSummary};
    use domain::error::JobLogError;
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use domain::user::UserId;
    use mocks::{MockJobLogStore, MockJobStore, MockReportRepo};

    use super::*;

    const DAY: SignedDuration = SignedDuration::from_hours(24);

    fn report(id: &str, age: SignedDuration) -> Report {
        Report {
            summary: ReportSummary {
                id: ReportId(id.into()),
                created_at: Timestamp::now().checked_sub(age).unwrap(),
                user: UserId("u1".into()),
                player: PlayerId("p1".into()),
                app: "flutter".into(),
                platform: "Android 14".into(),
                app_version: "0.0.7".into(),
                note: None,
            },
            payload: "{}".into(),
        }
    }

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
        logs.append(&JobId("old".into()), Timestamp::UNIX_EPOCH, LogLevel::Info, "a line")
            .await
            .unwrap();

        RetentionHandler::new(jobs.clone(), logs.clone(), MockReportRepo::new(), DAY, DAY)
            .handle(&retention_job())
            .await
            .unwrap();

        assert!(jobs.get(&JobId("old".into())).await.unwrap().is_none());
        assert!(logs.lines_for(&JobId("old".into())).is_empty());
    }

    #[tokio::test]
    async fn reports_past_their_own_window_are_removed() {
        let reports = MockReportRepo::new();
        reports.seed(report("expired", SignedDuration::from_hours(25)));
        reports.seed(report("kept", SignedDuration::from_hours(1)));
        let jobs = MockJobStore::new();
        seed(&jobs, "a-day-and-more", JobStatus::Succeeded, SignedDuration::from_hours(25)).await;

        let month = SignedDuration::from_hours(24 * 30);
        RetentionHandler::new(jobs.clone(), MockJobLogStore::new(), reports.clone(), month, DAY)
            .handle(&retention_job())
            .await
            .unwrap();

        let left = reports.list(Timestamp::UNIX_EPOCH, None, PageRequest::ALL).await.unwrap();
        let ids: Vec<&str> = left.items.iter().map(|r| r.id.0.as_str()).collect();
        assert_eq!(ids, ["kept"]);
        assert!(
            jobs.get(&JobId("a-day-and-more".into())).await.unwrap().is_some(),
            "the jobs keep their own window"
        );
    }

    #[tokio::test]
    async fn a_report_store_that_fails_makes_the_run_retryable_after_the_jobs_go() {
        let reports = MockReportRepo::new();
        reports.set_fail();
        let jobs = MockJobStore::new();
        seed(&jobs, "old", JobStatus::Succeeded, SignedDuration::from_hours(48)).await;

        let error = RetentionHandler::new(jobs.clone(), MockJobLogStore::new(), reports, DAY, DAY)
            .handle(&retention_job())
            .await
            .unwrap_err();

        assert!(matches!(error, JobError::Retryable(_)));
        assert!(jobs.get(&JobId("old".into())).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn recent_and_active_jobs_are_kept() {
        let jobs = MockJobStore::new();
        let logs = MockJobLogStore::new();
        seed(&jobs, "recent", JobStatus::Failed, SignedDuration::from_hours(1)).await;
        seed(&jobs, "queued", JobStatus::Queued, SignedDuration::from_hours(0)).await;
        seed(&jobs, "running", JobStatus::Running, SignedDuration::from_hours(0)).await;

        RetentionHandler::new(jobs.clone(), logs, MockReportRepo::new(), DAY, DAY)
            .handle(&retention_job())
            .await
            .unwrap();

        for id in ["recent", "queued", "running"] {
            assert!(jobs.get(&JobId(id.into())).await.unwrap().is_some());
        }
    }

    #[tokio::test]
    async fn a_window_too_long_to_subtract_keeps_everything_while_the_other_still_runs() {
        let max = SignedDuration::MAX;
        for (jobs_for, reports_for, jobs_left, reports_left) in [(DAY, max, 0, 1), (max, DAY, 1, 0)]
        {
            let jobs = MockJobStore::new();
            seed(&jobs, "old", JobStatus::Succeeded, SignedDuration::from_hours(48)).await;
            let reports = MockReportRepo::new();
            reports.seed(report("old", SignedDuration::from_hours(48)));

            RetentionHandler::new(
                jobs.clone(),
                MockJobLogStore::new(),
                reports.clone(),
                jobs_for,
                reports_for,
            )
            .handle(&retention_job())
            .await
            .unwrap();

            let kept = reports.list(Timestamp::UNIX_EPOCH, None, PageRequest::ALL).await.unwrap();
            assert_eq!(jobs.list().await.unwrap().len(), jobs_left, "jobs for {jobs_for:?}");
            assert_eq!(kept.total, reports_left, "reports for {reports_for:?}");
        }
    }

    #[tokio::test]
    async fn a_job_is_still_dropped_when_wiping_its_log_fails() {
        let jobs = MockJobStore::new();
        let logs = FailingWipe(MockJobLogStore::new());
        seed(&jobs, "old", JobStatus::Succeeded, SignedDuration::from_hours(48)).await;
        logs.append(&JobId("old".into()), Timestamp::UNIX_EPOCH, LogLevel::Info, "a line")
            .await
            .unwrap();

        RetentionHandler::new(jobs.clone(), logs.clone(), MockReportRepo::new(), DAY, DAY)
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
            level: LogLevel,
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

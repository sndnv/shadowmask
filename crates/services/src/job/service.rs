use std::sync::Arc;

use domain::common::{Page, PageRequest};
use domain::error::JobServiceError;
use domain::job::{Job, JobCanceller, JobId, JobKind, JobNode, JobPage, JobQuery, JobStatus};
use domain::repository::JobRepository;
use domain::service::JobService;
use domain::user::Principal;
use jiff::Timestamp;

use crate::acl;

struct NoopCanceller;

impl JobCanceller for NoopCanceller {
    fn request_cancel(&self, _id: &JobId) -> bool {
        false
    }
}

#[derive(Clone)]
pub struct JobServiceImpl<J> {
    jobs: J,
    canceller: Arc<dyn JobCanceller>,
    parked: Arc<[JobKind]>,
}

impl<J> JobServiceImpl<J> {
    pub fn new(jobs: J) -> Self {
        Self { jobs, canceller: Arc::new(NoopCanceller), parked: Arc::from([]) }
    }

    pub fn with_canceller(mut self, canceller: Arc<dyn JobCanceller>) -> Self {
        self.canceller = canceller;
        self
    }

    pub fn with_parked(mut self, parked: Vec<JobKind>) -> Self {
        self.parked = parked.into();
        self
    }
}

impl<J> JobService for JobServiceImpl<J>
where
    J: JobRepository + Sync,
{
    async fn jobs(
        &self,
        caller: &Principal,
        query: &JobQuery,
        page: PageRequest,
    ) -> Result<JobPage, JobServiceError> {
        if !acl::is_admin(caller) {
            return Err(JobServiceError::Forbidden);
        }
        let items = self.jobs.list_page(query, page).await?;
        let active_total =
            self.jobs.count(&JobQuery { search: query.search.clone(), active_only: true }).await?;
        let all_total =
            self.jobs.count(&JobQuery { search: query.search.clone(), active_only: false }).await?;
        Ok(JobPage {
            page: Page {
                items,
                total: if query.active_only { active_total } else { all_total },
                offset: page.offset,
                limit: page.limit,
            },
            active_total,
            all_total,
        })
    }

    async fn job_descendants(
        &self,
        caller: &Principal,
        id: &JobId,
        page: PageRequest,
    ) -> Result<Page<JobNode>, JobServiceError> {
        if !acl::is_admin(caller) {
            return Err(JobServiceError::Forbidden);
        }
        Ok(self.jobs.list_descendants(id, page).await?)
    }

    async fn job(&self, caller: &Principal, id: &JobId) -> Result<Option<Job>, JobServiceError> {
        if !acl::is_admin(caller) {
            return Err(JobServiceError::Forbidden);
        }
        Ok(self.jobs.get(id).await?)
    }

    async fn cancel_job(&self, caller: &Principal, id: &JobId) -> Result<(), JobServiceError> {
        if !acl::is_admin(caller) {
            return Err(JobServiceError::Forbidden);
        }
        if self.jobs.cancel(id, Timestamp::now()).await? {
            return Ok(());
        }
        let job = self.jobs.get(id).await?.ok_or(JobServiceError::NotFound)?;
        match job.status {
            JobStatus::Cancelled => Ok(()),
            JobStatus::Running if job.kind.is_process_killable() => {
                self.canceller.request_cancel(id);
                Ok(())
            }
            _ => Err(JobServiceError::NotCancellable),
        }
    }

    async fn retry_job(&self, caller: &Principal, id: &JobId) -> Result<Job, JobServiceError> {
        if !acl::is_admin(caller) {
            return Err(JobServiceError::Forbidden);
        }
        let job = self.jobs.get(id).await?.ok_or(JobServiceError::NotFound)?;
        if !self.is_retryable(&job) || !self.jobs.retry(id, Timestamp::now()).await? {
            return Err(JobServiceError::NotRetryable);
        }
        self.jobs.get(id).await?.ok_or(JobServiceError::NotFound)
    }

    fn is_retryable(&self, job: &Job) -> bool {
        matches!(job.status, JobStatus::Failed | JobStatus::Cancelled)
            && job.kind != JobKind::LibraryScan
            && !self.parked.contains(&job.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::job::JobPriority;
    use domain::user::{Role, UserId};
    use mocks::MockJobStore;

    const PAGE: PageRequest = PageRequest { offset: 0, limit: 50 };

    fn admin() -> Principal {
        Principal { user: UserId("admin".into()), role: Role::Admin }
    }

    fn member() -> Principal {
        Principal { user: UserId("u1".into()), role: Role::User }
    }

    fn job_with(id: &str, kind: JobKind, status: JobStatus) -> Job {
        let now = Timestamp::now();
        Job {
            id: JobId(id.into()),
            kind,
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
            finished_at: None,
            parent_id: None,
        }
    }

    #[derive(Clone, Default)]
    struct RecordingCanceller {
        calls: Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl JobCanceller for RecordingCanceller {
        fn request_cancel(&self, id: &JobId) -> bool {
            self.calls.lock().unwrap().push(id.0.clone());
            true
        }
    }

    fn svc() -> JobServiceImpl<MockJobStore> {
        JobServiceImpl::new(MockJobStore::new())
    }

    #[tokio::test]
    async fn jobs_admin_only() {
        let svc = svc();
        svc.jobs.enqueue(job_with("j1", JobKind::LibraryScan, JobStatus::Queued)).await.unwrap();
        let page = svc.jobs(&admin(), &JobQuery::default(), PAGE).await.unwrap();
        assert_eq!(page.page.items.len(), 1);
        assert_eq!(page.all_total, 1);
        assert_eq!(page.active_total, 1);
        assert!(matches!(
            svc.jobs(&member(), &JobQuery::default(), PAGE).await.unwrap_err(),
            JobServiceError::Forbidden
        ));
    }

    #[tokio::test]
    async fn jobs_filters_and_pages_with_both_tab_counts() {
        let svc = svc();
        for (id, kind, status) in [
            ("j1", JobKind::LibraryScan, JobStatus::Queued),
            ("j2", JobKind::Artwork, JobStatus::Running),
            ("j3", JobKind::Artwork, JobStatus::Succeeded),
        ] {
            svc.jobs.enqueue(job_with(id, kind, status)).await.unwrap();
        }

        let active = svc.jobs(&admin(), &JobQuery::active(true), PAGE).await.unwrap();
        assert_eq!(active.page.total, 2);
        assert_eq!(active.active_total, 2);
        assert_eq!(active.all_total, 3);

        let artwork = svc
            .jobs(&admin(), &JobQuery { search: Some("artwork".into()), active_only: false }, PAGE)
            .await
            .unwrap();
        assert_eq!(artwork.page.total, 2);
        assert_eq!(artwork.active_total, 1);
        assert_eq!(artwork.all_total, 2);

        let second = svc
            .jobs(&admin(), &JobQuery::default(), PageRequest { offset: 2, limit: 2 })
            .await
            .unwrap();
        assert_eq!(second.page.items.len(), 1);
        assert_eq!(second.page.total, 3);
    }

    #[tokio::test]
    async fn job_descendants_admin_only() {
        let svc = svc();
        svc.jobs.enqueue(job_with("root", JobKind::LibraryScan, JobStatus::Running)).await.unwrap();
        let mut child = job_with("child", JobKind::Artwork, JobStatus::Queued);
        child.parent_id = Some(JobId("root".into()));
        svc.jobs.enqueue(child).await.unwrap();

        let tree = svc.job_descendants(&admin(), &JobId("root".into()), PAGE).await.unwrap();
        assert_eq!(tree.total, 1);
        assert_eq!(tree.items[0].job.id.0, "child");
        assert_eq!(tree.items[0].depth, 0);
        assert!(matches!(
            svc.job_descendants(&member(), &JobId("root".into()), PAGE).await.unwrap_err(),
            JobServiceError::Forbidden
        ));
    }

    #[tokio::test]
    async fn job_admin_only() {
        let svc = svc();
        svc.jobs.enqueue(job_with("j1", JobKind::LibraryScan, JobStatus::Queued)).await.unwrap();
        assert_eq!(
            svc.job(&admin(), &JobId("j1".into())).await.unwrap().unwrap().id,
            JobId("j1".into())
        );
        assert!(svc.job(&admin(), &JobId("nope".into())).await.unwrap().is_none());
        assert!(matches!(
            svc.job(&member(), &JobId("j1".into())).await.unwrap_err(),
            JobServiceError::Forbidden
        ));
    }

    #[tokio::test]
    async fn cancel_job_requires_admin() {
        let svc = svc();
        assert!(matches!(
            svc.cancel_job(&member(), &JobId("x".into())).await.unwrap_err(),
            JobServiceError::Forbidden
        ));
    }

    #[tokio::test]
    async fn cancel_job_queued_marks_cancelled() {
        let svc = svc();
        svc.jobs.enqueue(job_with("q", JobKind::LibraryScan, JobStatus::Queued)).await.unwrap();
        svc.cancel_job(&admin(), &JobId("q".into())).await.unwrap();
        let stored = svc.jobs.get(&JobId("q".into())).await.unwrap().unwrap();
        assert_eq!(stored.status, JobStatus::Cancelled);
        assert!(stored.finished_at.is_some());
    }

    #[tokio::test]
    async fn cancel_job_missing_is_not_found() {
        let svc = svc();
        assert!(matches!(
            svc.cancel_job(&admin(), &JobId("nope".into())).await.unwrap_err(),
            JobServiceError::NotFound
        ));
    }

    #[tokio::test]
    async fn cancel_job_running_killable_signals_canceller() {
        let jobs = MockJobStore::new();
        jobs.enqueue(job_with("f", JobKind::Fetch, JobStatus::Running)).await.unwrap();
        let canceller = RecordingCanceller::default();
        let svc = JobServiceImpl::new(jobs).with_canceller(Arc::new(canceller.clone()));
        svc.cancel_job(&admin(), &JobId("f".into())).await.unwrap();
        let calls = canceller.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], "f");
    }

    #[tokio::test]
    async fn cancel_job_running_killable_default_canceller_is_ok() {
        let svc = svc();
        svc.jobs.enqueue(job_with("t", JobKind::Trickplay, JobStatus::Running)).await.unwrap();
        svc.cancel_job(&admin(), &JobId("t".into())).await.unwrap();
    }

    #[tokio::test]
    async fn cancel_job_running_non_killable_conflicts() {
        let svc = svc();
        svc.jobs.enqueue(job_with("s", JobKind::LibraryScan, JobStatus::Running)).await.unwrap();
        assert!(matches!(
            svc.cancel_job(&admin(), &JobId("s".into())).await.unwrap_err(),
            JobServiceError::NotCancellable
        ));
    }

    #[tokio::test]
    async fn cancel_job_already_cancelled_is_idempotent() {
        let svc = svc();
        svc.jobs.enqueue(job_with("c", JobKind::Fetch, JobStatus::Cancelled)).await.unwrap();
        svc.cancel_job(&admin(), &JobId("c".into())).await.unwrap();
    }

    #[tokio::test]
    async fn cancel_job_terminal_conflicts() {
        let svc = svc();
        svc.jobs.enqueue(job_with("d", JobKind::Fetch, JobStatus::Succeeded)).await.unwrap();
        assert!(matches!(
            svc.cancel_job(&admin(), &JobId("d".into())).await.unwrap_err(),
            JobServiceError::NotCancellable
        ));
    }

    #[test]
    fn only_a_failed_or_cancelled_job_of_a_live_kind_is_retryable() {
        let svc = svc().with_parked(vec![JobKind::Transcription]);
        let cases = [
            (JobKind::Subtitles, JobStatus::Failed, true),
            (JobKind::Subtitles, JobStatus::Cancelled, true),
            (JobKind::Subtitles, JobStatus::Queued, false),
            (JobKind::Subtitles, JobStatus::Running, false),
            (JobKind::Subtitles, JobStatus::Succeeded, false),
            (JobKind::Transcription, JobStatus::Failed, false),
            (JobKind::LibraryScan, JobStatus::Failed, false),
            (JobKind::LibraryScan, JobStatus::Cancelled, false),
        ];
        for (kind, status, expected) in cases {
            assert_eq!(
                svc.is_retryable(&job_with("j", kind, status)),
                expected,
                "{kind:?} {status:?}"
            );
        }
    }

    #[tokio::test]
    async fn retry_job_requeues_a_failed_job() {
        let svc = svc();
        let mut failed = job_with("f", JobKind::Subtitles, JobStatus::Failed);
        failed.attempts = 3;
        failed.last_error = Some("http status 406 Not Acceptable".into());
        failed.finished_at = Some(Timestamp::now());
        svc.jobs.enqueue(failed).await.unwrap();

        let job = svc.retry_job(&admin(), &JobId("f".into())).await.unwrap();

        assert_eq!(job.status, JobStatus::Queued);
        assert_eq!(job.attempts, 0);
        assert_eq!(job.last_error, None);
        assert_eq!(job.finished_at, None);
    }

    #[tokio::test]
    async fn retry_job_requires_admin() {
        let svc = svc();
        svc.jobs.enqueue(job_with("f", JobKind::Subtitles, JobStatus::Failed)).await.unwrap();
        assert!(matches!(
            svc.retry_job(&member(), &JobId("f".into())).await.unwrap_err(),
            JobServiceError::Forbidden
        ));
        assert_eq!(
            svc.jobs.get(&JobId("f".into())).await.unwrap().unwrap().status,
            JobStatus::Failed
        );
    }

    #[tokio::test]
    async fn retry_job_missing_is_not_found() {
        assert!(matches!(
            svc().retry_job(&admin(), &JobId("nope".into())).await.unwrap_err(),
            JobServiceError::NotFound
        ));
    }

    #[tokio::test]
    async fn retry_job_refuses_a_parked_kind_and_a_finished_job() {
        let svc = svc().with_parked(vec![JobKind::Transcription]);
        svc.jobs.enqueue(job_with("t", JobKind::Transcription, JobStatus::Failed)).await.unwrap();
        svc.jobs.enqueue(job_with("s", JobKind::Subtitles, JobStatus::Succeeded)).await.unwrap();
        for id in ["t", "s"] {
            assert!(matches!(
                svc.retry_job(&admin(), &JobId(id.into())).await.unwrap_err(),
                JobServiceError::NotRetryable
            ));
        }
        assert_eq!(
            svc.jobs.get(&JobId("t".into())).await.unwrap().unwrap().status,
            JobStatus::Failed,
            "a parked kind would sit queued forever, so it stays failed"
        );
    }

    #[tokio::test]
    async fn a_failed_library_scan_is_scanned_again_rather_than_retried() {
        let svc = svc();
        svc.jobs.enqueue(job_with("l", JobKind::LibraryScan, JobStatus::Failed)).await.unwrap();

        assert!(matches!(
            svc.retry_job(&admin(), &JobId("l".into())).await.unwrap_err(),
            JobServiceError::NotRetryable
        ));
        assert_eq!(
            svc.jobs.get(&JobId("l".into())).await.unwrap().unwrap().status,
            JobStatus::Failed,
            "a re-queued scan would skip the library's scan slot"
        );
    }
}

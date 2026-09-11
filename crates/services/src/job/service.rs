use std::sync::Arc;

use domain::common::{Page, PageRequest};
use domain::error::JobServiceError;
use domain::job::{Job, JobCanceller, JobId, JobNode, JobPage, JobQuery, JobStatus};
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
}

impl<J> JobServiceImpl<J> {
    pub fn new(jobs: J) -> Self {
        Self { jobs, canceller: Arc::new(NoopCanceller) }
    }

    pub fn with_canceller(mut self, canceller: Arc<dyn JobCanceller>) -> Self {
        self.canceller = canceller;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::job::{JobKind, JobPriority};
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
}

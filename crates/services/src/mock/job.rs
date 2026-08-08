use std::sync::{Arc, Mutex};

use domain::error::JobServiceError;
use domain::job::{Job, JobId};
use domain::service::JobService;
use domain::user::Principal;

#[derive(Clone, Default)]
pub struct MockJobService {
    jobs: Arc<Mutex<Vec<Job>>>,
}

impl MockJobService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_job(&self, job: Job) {
        self.jobs.lock().unwrap().push(job);
    }
}

impl JobService for MockJobService {
    async fn jobs(&self, _caller: &Principal) -> Result<Vec<Job>, JobServiceError> {
        Ok(self.jobs.lock().unwrap().clone())
    }

    async fn job(&self, _caller: &Principal, id: &JobId) -> Result<Option<Job>, JobServiceError> {
        Ok(self
            .jobs
            .lock()
            .unwrap()
            .iter()
            .find(|j| &j.id == id)
            .cloned())
    }

    async fn cancel_job(&self, _caller: &Principal, _id: &JobId) -> Result<(), JobServiceError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::job::{JobKind, JobPriority, JobStatus};
    use domain::user::{Role, UserId};
    use jiff::Timestamp;

    fn principal() -> Principal {
        Principal {
            user: UserId("u1".into()),
            role: Role::Admin,
        }
    }

    #[tokio::test]
    async fn seeded_jobs_are_returned_and_cancellable() {
        let svc = MockJobService::new();
        assert!(svc.jobs(&principal()).await.unwrap().is_empty());
        let now = Timestamp::UNIX_EPOCH;
        svc.add_job(Job {
            id: JobId("j1".into()),
            kind: JobKind::LibraryScan,
            status: JobStatus::Succeeded,
            priority: JobPriority::Normal,
            payload: "lib1".into(),
            attempts: 1,
            progress: 1.0,
            available_at: now,
            last_error: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            finished_at: None,
            parent_id: None,
        });
        assert_eq!(svc.jobs(&principal()).await.unwrap().len(), 1);
        assert!(
            svc.job(&principal(), &JobId("j1".into()))
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            svc.job(&principal(), &JobId("x".into()))
                .await
                .unwrap()
                .is_none()
        );
        svc.cancel_job(&principal(), &JobId("j1".into()))
            .await
            .unwrap();
    }
}

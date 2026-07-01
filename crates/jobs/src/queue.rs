use std::sync::atomic::{AtomicU64, Ordering};

use domain::error::RepositoryError;
use domain::job::{Job, JobId, JobKind, JobPriority, JobStatus};
use domain::repository::JobRepository;
use jiff::Timestamp;

pub struct JobQueue<R> {
    repo: R,
    counter: AtomicU64,
}

impl<R: JobRepository> JobQueue<R> {
    pub fn new(repo: R) -> Self {
        Self {
            repo,
            counter: AtomicU64::new(0),
        }
    }

    pub async fn enqueue(
        &self,
        kind: JobKind,
        priority: JobPriority,
        payload: String,
        now: Timestamp,
    ) -> Result<JobId, RepositoryError> {
        let sequence = self.counter.fetch_add(1, Ordering::Relaxed) + 1;
        let id = JobId(format!("job-{sequence}"));
        let job = Job {
            id: id.clone(),
            kind,
            status: JobStatus::Queued,
            priority,
            payload,
            attempts: 0,
            progress: 0.0,
            available_at: now,
            last_error: None,
            created_at: now,
            updated_at: now,
        };
        self.repo.enqueue(job).await?;
        Ok(id)
    }

    pub async fn status(&self, id: &JobId) -> Result<Option<Job>, RepositoryError> {
        self.repo.get(id).await
    }

    pub async fn list(&self) -> Result<Vec<Job>, RepositoryError> {
        self.repo.list().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use services::mock::MockJobStore;

    #[tokio::test]
    async fn enqueue_builds_queued_job() {
        let now = Timestamp::now();
        let queue = JobQueue::new(MockJobStore::new());

        let id = queue
            .enqueue(JobKind::LibraryScan, JobPriority::Normal, "p".into(), now)
            .await
            .unwrap();
        assert_eq!(id, JobId("job-1".into()));

        let job = queue.status(&id).await.unwrap().unwrap();
        assert_eq!(job.status, JobStatus::Queued);
        assert_eq!(job.attempts, 0);
        assert_eq!(job.progress, 0.0);
        assert_eq!(job.available_at, now);
        assert_eq!(job.payload, "p");
    }

    #[tokio::test]
    async fn enqueue_increments_ids_and_lists() {
        let now = Timestamp::now();
        let queue = JobQueue::new(MockJobStore::new());

        let first = queue
            .enqueue(JobKind::Metadata, JobPriority::Low, String::new(), now)
            .await
            .unwrap();
        let second = queue
            .enqueue(JobKind::Artwork, JobPriority::High, String::new(), now)
            .await
            .unwrap();
        assert_eq!(first, JobId("job-1".into()));
        assert_eq!(second, JobId("job-2".into()));
        assert_eq!(queue.list().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn status_of_unknown_is_none() {
        let queue = JobQueue::new(MockJobStore::new());
        assert!(
            queue
                .status(&JobId("missing".into()))
                .await
                .unwrap()
                .is_none()
        );
    }
}

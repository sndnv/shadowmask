use domain::error::RepositoryError;
use domain::job::{Job, JobId, JobKind, JobPriority, JobStatus};
use domain::repository::JobRepository;
use jiff::Timestamp;
use uuid::Uuid;

pub struct JobQueue<R> {
    repo: R,
}

impl<R: JobRepository> JobQueue<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn enqueue(
        &self,
        kind: JobKind,
        priority: JobPriority,
        payload: String,
        now: Timestamp,
    ) -> Result<JobId, RepositoryError> {
        let id = JobId(Uuid::new_v4().to_string());
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
            started_at: None,
            finished_at: None,
            parent_id: None,
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
        assert!(!id.0.is_empty());

        let job = queue.status(&id).await.unwrap().unwrap();
        assert_eq!(job.status, JobStatus::Queued);
        assert_eq!(job.attempts, 0);
        assert_eq!(job.progress, 0.0);
        assert_eq!(job.available_at, now);
        assert_eq!(job.payload, "p");
    }

    #[tokio::test]
    async fn enqueue_generates_unique_ids_and_lists() {
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
        assert_ne!(first, second);
        assert!(!first.0.is_empty() && !second.0.is_empty());
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

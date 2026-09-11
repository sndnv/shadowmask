use domain::error::RepositoryError;
use domain::job::{Job, JobId, JobKind, JobPriority};
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
        let job = Job::queued(id.clone(), kind, priority, payload, None, now);
        self.repo.enqueue(job).await?;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::job::JobStatus;
    use mocks::MockJobStore;

    #[tokio::test]
    async fn enqueue_builds_queued_job() {
        let now = Timestamp::now();
        let store = MockJobStore::new();
        let queue = JobQueue::new(store.clone());

        let id = queue
            .enqueue(JobKind::LibraryScan, JobPriority::Normal, "p".into(), now)
            .await
            .unwrap();
        assert!(!id.0.is_empty());

        let job = store.get(&id).await.unwrap().unwrap();
        assert_eq!(job.status, JobStatus::Queued);
        assert_eq!(job.attempts, 0);
        assert_eq!(job.progress, 0.0);
        assert_eq!(job.available_at, now);
        assert_eq!(job.payload, "p");
    }

    #[tokio::test]
    async fn enqueue_generates_unique_ids_and_lists() {
        let now = Timestamp::now();
        let store = MockJobStore::new();
        let queue = JobQueue::new(store.clone());

        let first =
            queue.enqueue(JobKind::Metadata, JobPriority::Low, String::new(), now).await.unwrap();
        let second =
            queue.enqueue(JobKind::Artwork, JobPriority::High, String::new(), now).await.unwrap();
        assert_ne!(first, second);
        assert!(!first.0.is_empty() && !second.0.is_empty());
        assert_eq!(store.list().await.unwrap().len(), 2);
    }
}

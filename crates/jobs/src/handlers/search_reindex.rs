use domain::job::Job;
use domain::repository::SearchIndex;

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct SearchReindexHandler<S> {
    index: S,
}

impl<S> SearchReindexHandler<S> {
    pub fn new(index: S) -> Self {
        Self { index }
    }
}

impl<S> JobHandler for SearchReindexHandler<S>
where
    S: SearchIndex + Send + Sync,
{
    async fn handle(&self, _job: &Job) -> Result<(), JobError> {
        tracing::info!("rebuilding search index");
        self.index.rebuild().await.map_err(|err| JobError::Retryable(err.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use domain::job::{Job, JobId, JobKind, JobPriority, JobStatus};
    use jiff::Timestamp;
    use mocks::MockSearchIndex;

    use super::*;

    fn reindex_job() -> Job {
        let now = Timestamp::UNIX_EPOCH;
        Job {
            id: JobId("job-1".into()),
            kind: JobKind::SearchReindex,
            status: JobStatus::Running,
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

    #[tokio::test]
    async fn rebuilds_the_index() {
        let index = MockSearchIndex::new();
        let handler = SearchReindexHandler::new(index.clone());

        handler.handle(&reindex_job()).await.unwrap();

        assert_eq!(index.rebuild_count(), 1);
    }

    #[tokio::test]
    async fn rebuild_failure_is_retryable() {
        let index = MockSearchIndex::new();
        index.set_fail();
        let handler = SearchReindexHandler::new(index);

        assert!(matches!(
            handler.handle(&reindex_job()).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }
}

use std::time::Instant;

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
        let started = Instant::now();
        let entries =
            self.index.rebuild().await.map_err(|err| JobError::Retryable(err.to_string()))?;
        let seconds = started.elapsed().as_secs_f64();
        tracing::info!("search index rebuilt: [{entries}] entries in [{seconds:.1} s]");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use domain::discovery::SearchResult;
    use domain::job::{Job, JobId, JobKind, JobPriority, JobStatus};
    use domain::metadata::{Person, PersonId};
    use jiff::Timestamp;
    use mocks::MockSearchIndex;
    use tracing_test::traced_test;

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

    #[traced_test]
    #[tokio::test]
    async fn rebuilds_the_index_and_logs_its_size() {
        let index = MockSearchIndex::new();
        index.add(SearchResult::Person(Person {
            id: PersonId("p1".into()),
            name: "Ada".into(),
            ..Person::default()
        }));
        let handler = SearchReindexHandler::new(index.clone());

        handler.handle(&reindex_job()).await.unwrap();

        assert_eq!(index.rebuild_count(), 1);
        assert!(logs_contain("search index rebuilt: [1] entries in ["));
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

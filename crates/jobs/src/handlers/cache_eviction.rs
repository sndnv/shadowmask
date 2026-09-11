use domain::job::Job;
use domain::media::TranscodeCacheMaintenance;

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct CacheEvictionHandler<C> {
    cache: C,
    max_bytes: u64,
}

impl<C> CacheEvictionHandler<C> {
    pub fn new(cache: C, max_bytes: u64) -> Self {
        Self { cache, max_bytes }
    }
}

impl<C> JobHandler for CacheEvictionHandler<C>
where
    C: TranscodeCacheMaintenance + Send + Sync,
{
    async fn handle(&self, _job: &Job) -> Result<(), JobError> {
        let evicted = self
            .cache
            .evict(self.max_bytes)
            .await
            .map_err(|err| JobError::Retryable(err.to_string()))?;
        tracing::info!(evicted, "transcode cache eviction complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use domain::error::CacheError;
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use jiff::Timestamp;

    use super::*;

    #[derive(Default)]
    struct StubCache {
        calls: Mutex<Vec<u64>>,
        fail: bool,
    }

    impl TranscodeCacheMaintenance for StubCache {
        async fn evict(&self, max_bytes: u64) -> Result<u64, CacheError> {
            self.calls.lock().unwrap().push(max_bytes);
            if self.fail { Err(CacheError::Io("boom".into())) } else { Ok(3) }
        }
    }

    fn eviction_job() -> Job {
        let now = Timestamp::UNIX_EPOCH;
        Job {
            id: JobId("job-1".into()),
            kind: JobKind::CacheEviction,
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
    async fn evicts_with_the_configured_cap() {
        let handler = CacheEvictionHandler::new(StubCache::default(), 4096);
        handler.handle(&eviction_job()).await.unwrap();
        assert_eq!(handler.cache.calls.lock().unwrap().as_slice(), [4096]);
    }

    #[tokio::test]
    async fn eviction_failure_is_retryable() {
        let cache = StubCache { fail: true, ..StubCache::default() };
        let handler = CacheEvictionHandler::new(cache, 0);
        assert!(matches!(
            handler.handle(&eviction_job()).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }
}

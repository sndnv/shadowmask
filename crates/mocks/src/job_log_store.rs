use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use domain::error::JobLogError;
use domain::job::{JobId, JobLogLevel, JobLogStore};
use jiff::Timestamp;

#[derive(Clone, Default)]
pub struct MockJobLogStore {
    lines: Arc<Mutex<HashMap<JobId, Vec<String>>>>,
}

impl MockJobLogStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn lines_for(&self, job: &JobId) -> Vec<String> {
        self.lines
            .lock()
            .unwrap()
            .get(job)
            .cloned()
            .unwrap_or_default()
    }
}

impl JobLogStore for MockJobLogStore {
    async fn append(
        &self,
        job: &JobId,
        at: Timestamp,
        level: JobLogLevel,
        message: &str,
    ) -> Result<(), JobLogError> {
        self.lines
            .lock()
            .unwrap()
            .entry(job.clone())
            .or_default()
            .push(format!("{at} {} {message}", level.as_str()));
        Ok(())
    }

    async fn read(&self, job: &JobId, tail: Option<usize>) -> Result<Vec<String>, JobLogError> {
        let mut lines = self.lines_for(job);
        if let Some(limit) = tail
            && lines.len() > limit
        {
            lines = lines.split_off(lines.len() - limit);
        }
        Ok(lines)
    }

    async fn wipe(&self, job: &JobId) -> Result<(), JobLogError> {
        self.lines.lock().unwrap().remove(job);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn append_read_tail_and_wipe() {
        let store = MockJobLogStore::new();
        let job = JobId("job-1".into());
        for n in 0..3 {
            store
                .append(
                    &job,
                    Timestamp::UNIX_EPOCH,
                    JobLogLevel::Info,
                    &format!("l{n}"),
                )
                .await
                .unwrap();
        }
        assert_eq!(store.read(&job, None).await.unwrap().len(), 3);
        assert_eq!(store.lines_for(&job).len(), 3);

        let tail = store.read(&job, Some(1)).await.unwrap();
        assert_eq!(tail.len(), 1);
        assert!(tail[0].ends_with("l2"));

        store.wipe(&job).await.unwrap();
        assert!(store.read(&job, None).await.unwrap().is_empty());
    }
}

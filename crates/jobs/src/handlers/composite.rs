use domain::job::{Job, JobKind};

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct CompositeJobHandler<Scan, Reindex> {
    scan: Scan,
    reindex: Reindex,
}

impl<Scan, Reindex> CompositeJobHandler<Scan, Reindex> {
    pub fn new(scan: Scan, reindex: Reindex) -> Self {
        Self { scan, reindex }
    }
}

impl<Scan, Reindex> JobHandler for CompositeJobHandler<Scan, Reindex>
where
    Scan: JobHandler + Send + Sync,
    Reindex: JobHandler + Send + Sync,
{
    async fn handle(&self, job: &Job) -> Result<(), JobError> {
        match job.kind {
            JobKind::LibraryScan => self.scan.handle(job).await,
            JobKind::SearchReindex => self.reindex.handle(job).await,
            other => Err(JobError::Permanent(format!(
                "no handler for job kind: {other:?}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use domain::job::{JobId, JobPriority, JobStatus};
    use jiff::Timestamp;

    use super::*;

    #[derive(Clone)]
    struct Recorder {
        tag: &'static str,
        calls: Arc<Mutex<Vec<&'static str>>>,
    }

    impl Recorder {
        fn new(tag: &'static str, calls: Arc<Mutex<Vec<&'static str>>>) -> Self {
            Self { tag, calls }
        }
    }

    impl JobHandler for Recorder {
        async fn handle(&self, _job: &Job) -> Result<(), JobError> {
            self.calls.lock().unwrap().push(self.tag);
            Ok(())
        }
    }

    fn job(kind: JobKind) -> Job {
        let now = Timestamp::UNIX_EPOCH;
        Job {
            id: JobId("job-1".into()),
            kind,
            status: JobStatus::Running,
            priority: JobPriority::Normal,
            payload: String::new(),
            attempts: 0,
            progress: 0.0,
            available_at: now,
            last_error: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn composite(calls: &Arc<Mutex<Vec<&'static str>>>) -> CompositeJobHandler<Recorder, Recorder> {
        CompositeJobHandler::new(
            Recorder::new("scan", Arc::clone(calls)),
            Recorder::new("reindex", Arc::clone(calls)),
        )
    }

    #[tokio::test]
    async fn routes_library_scan_to_scan_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls)
            .handle(&job(JobKind::LibraryScan))
            .await
            .unwrap();
        assert_eq!(*calls.lock().unwrap(), ["scan"]);
    }

    #[tokio::test]
    async fn routes_search_reindex_to_reindex_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls)
            .handle(&job(JobKind::SearchReindex))
            .await
            .unwrap();
        assert_eq!(*calls.lock().unwrap(), ["reindex"]);
    }

    #[tokio::test]
    async fn unhandled_kind_is_permanent_and_routes_nowhere() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let error = composite(&calls)
            .handle(&job(JobKind::Metadata))
            .await
            .unwrap_err();
        assert!(matches!(error, JobError::Permanent(_)));
        assert!(calls.lock().unwrap().is_empty());
    }
}

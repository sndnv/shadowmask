use domain::job::{Job, JobKind};

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct CompositeJobHandler<Scan, Reindex, Artwork, Trickplay, Ingest, Metadata> {
    scan: Scan,
    reindex: Reindex,
    artwork: Artwork,
    trickplay: Trickplay,
    ingest: Ingest,
    metadata: Metadata,
}

impl<Scan, Reindex, Artwork, Trickplay, Ingest, Metadata>
    CompositeJobHandler<Scan, Reindex, Artwork, Trickplay, Ingest, Metadata>
{
    pub fn new(
        scan: Scan,
        reindex: Reindex,
        artwork: Artwork,
        trickplay: Trickplay,
        ingest: Ingest,
        metadata: Metadata,
    ) -> Self {
        Self {
            scan,
            reindex,
            artwork,
            trickplay,
            ingest,
            metadata,
        }
    }
}

impl<Scan, Reindex, Artwork, Trickplay, Ingest, Metadata> JobHandler
    for CompositeJobHandler<Scan, Reindex, Artwork, Trickplay, Ingest, Metadata>
where
    Scan: JobHandler + Send + Sync,
    Reindex: JobHandler + Send + Sync,
    Artwork: JobHandler + Send + Sync,
    Trickplay: JobHandler + Send + Sync,
    Ingest: JobHandler + Send + Sync,
    Metadata: JobHandler + Send + Sync,
{
    async fn handle(&self, job: &Job) -> Result<(), JobError> {
        match job.kind {
            JobKind::LibraryScan => self.scan.handle(job).await,
            JobKind::SearchReindex => self.reindex.handle(job).await,
            JobKind::Artwork => self.artwork.handle(job).await,
            JobKind::Trickplay => self.trickplay.handle(job).await,
            JobKind::Ingest => self.ingest.handle(job).await,
            JobKind::Metadata => self.metadata.handle(job).await,
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

    fn composite(
        calls: &Arc<Mutex<Vec<&'static str>>>,
    ) -> CompositeJobHandler<Recorder, Recorder, Recorder, Recorder, Recorder, Recorder> {
        CompositeJobHandler::new(
            Recorder::new("scan", Arc::clone(calls)),
            Recorder::new("reindex", Arc::clone(calls)),
            Recorder::new("artwork", Arc::clone(calls)),
            Recorder::new("trickplay", Arc::clone(calls)),
            Recorder::new("ingest", Arc::clone(calls)),
            Recorder::new("metadata", Arc::clone(calls)),
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
    async fn routes_artwork_to_artwork_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls)
            .handle(&job(JobKind::Artwork))
            .await
            .unwrap();
        assert_eq!(*calls.lock().unwrap(), ["artwork"]);
    }

    #[tokio::test]
    async fn routes_trickplay_to_trickplay_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls)
            .handle(&job(JobKind::Trickplay))
            .await
            .unwrap();
        assert_eq!(*calls.lock().unwrap(), ["trickplay"]);
    }

    #[tokio::test]
    async fn routes_ingest_to_ingest_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls)
            .handle(&job(JobKind::Ingest))
            .await
            .unwrap();
        assert_eq!(*calls.lock().unwrap(), ["ingest"]);
    }

    #[tokio::test]
    async fn routes_metadata_to_metadata_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls)
            .handle(&job(JobKind::Metadata))
            .await
            .unwrap();
        assert_eq!(*calls.lock().unwrap(), ["metadata"]);
    }

    #[tokio::test]
    async fn unhandled_kind_is_permanent_and_routes_nowhere() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let error = composite(&calls)
            .handle(&job(JobKind::Subtitles))
            .await
            .unwrap_err();
        assert!(matches!(error, JobError::Permanent(_)));
        assert!(calls.lock().unwrap().is_empty());
    }
}

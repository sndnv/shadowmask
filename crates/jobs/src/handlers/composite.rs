use domain::job::{Job, JobKind};

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct CompositeJobHandler<
    Scan,
    Reindex,
    Artwork,
    Trickplay,
    Ingest,
    Metadata,
    Relink,
    Subtitles,
    Transcription,
    Translation,
    Upscale,
    Combine,
    Fetch,
    Eviction,
    Nightly,
    Retention,
    Sweep,
> {
    scan: Scan,
    reindex: Reindex,
    artwork: Artwork,
    trickplay: Trickplay,
    ingest: Ingest,
    metadata: Metadata,
    relink: Relink,
    subtitles: Subtitles,
    transcription: Transcription,
    translation: Translation,
    upscale: Upscale,
    combine: Combine,
    fetch: Fetch,
    eviction: Eviction,
    nightly: Nightly,
    retention: Retention,
    sweep: Sweep,
}

impl<
    Scan,
    Reindex,
    Artwork,
    Trickplay,
    Ingest,
    Metadata,
    Relink,
    Subtitles,
    Transcription,
    Translation,
    Upscale,
    Combine,
    Fetch,
    Eviction,
    Nightly,
    Retention,
    Sweep,
>
    CompositeJobHandler<
        Scan,
        Reindex,
        Artwork,
        Trickplay,
        Ingest,
        Metadata,
        Relink,
        Subtitles,
        Transcription,
        Translation,
        Upscale,
        Combine,
        Fetch,
        Eviction,
        Nightly,
        Retention,
        Sweep,
    >
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        scan: Scan,
        reindex: Reindex,
        artwork: Artwork,
        trickplay: Trickplay,
        ingest: Ingest,
        metadata: Metadata,
        relink: Relink,
        subtitles: Subtitles,
        transcription: Transcription,
        translation: Translation,
        upscale: Upscale,
        combine: Combine,
        fetch: Fetch,
        eviction: Eviction,
        nightly: Nightly,
        retention: Retention,
        sweep: Sweep,
    ) -> Self {
        Self {
            scan,
            reindex,
            artwork,
            trickplay,
            ingest,
            metadata,
            relink,
            subtitles,
            transcription,
            translation,
            upscale,
            combine,
            fetch,
            eviction,
            nightly,
            retention,
            sweep,
        }
    }
}

impl<
    Scan,
    Reindex,
    Artwork,
    Trickplay,
    Ingest,
    Metadata,
    Relink,
    Subtitles,
    Transcription,
    Translation,
    Upscale,
    Combine,
    Fetch,
    Eviction,
    Nightly,
    Retention,
    Sweep,
> JobHandler
    for CompositeJobHandler<
        Scan,
        Reindex,
        Artwork,
        Trickplay,
        Ingest,
        Metadata,
        Relink,
        Subtitles,
        Transcription,
        Translation,
        Upscale,
        Combine,
        Fetch,
        Eviction,
        Nightly,
        Retention,
        Sweep,
    >
where
    Scan: JobHandler + Send + Sync,
    Reindex: JobHandler + Send + Sync,
    Artwork: JobHandler + Send + Sync,
    Trickplay: JobHandler + Send + Sync,
    Ingest: JobHandler + Send + Sync,
    Metadata: JobHandler + Send + Sync,
    Relink: JobHandler + Send + Sync,
    Subtitles: JobHandler + Send + Sync,
    Transcription: JobHandler + Send + Sync,
    Translation: JobHandler + Send + Sync,
    Upscale: JobHandler + Send + Sync,
    Combine: JobHandler + Send + Sync,
    Fetch: JobHandler + Send + Sync,
    Eviction: JobHandler + Send + Sync,
    Nightly: JobHandler + Send + Sync,
    Retention: JobHandler + Send + Sync,
    Sweep: JobHandler + Send + Sync,
{
    async fn handle(&self, job: &Job) -> Result<(), JobError> {
        match job.kind {
            JobKind::LibraryScan => self.scan.handle(job).await,
            JobKind::SearchReindex => self.reindex.handle(job).await,
            JobKind::Artwork => self.artwork.handle(job).await,
            JobKind::Trickplay => self.trickplay.handle(job).await,
            JobKind::Ingest => self.ingest.handle(job).await,
            JobKind::Metadata => self.metadata.handle(job).await,
            JobKind::Relink => self.relink.handle(job).await,
            JobKind::Subtitles => self.subtitles.handle(job).await,
            JobKind::Transcription => self.transcription.handle(job).await,
            JobKind::Translation => self.translation.handle(job).await,
            JobKind::Upscale => self.upscale.handle(job).await,
            JobKind::Combine => self.combine.handle(job).await,
            JobKind::Fetch => self.fetch.handle(job).await,
            JobKind::CacheEviction => self.eviction.handle(job).await,
            JobKind::ScheduledScan => self.nightly.handle(job).await,
            JobKind::Retention => self.retention.handle(job).await,
            JobKind::OrphanSweep => self.sweep.handle(job).await,
            other => Err(JobError::Permanent(format!("no handler for job kind: {other:?}"))),
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
            started_at: None,
            finished_at: None,
            parent_id: None,
        }
    }

    #[allow(clippy::type_complexity)]
    fn composite(
        calls: &Arc<Mutex<Vec<&'static str>>>,
    ) -> CompositeJobHandler<
        Recorder,
        Recorder,
        Recorder,
        Recorder,
        Recorder,
        Recorder,
        Recorder,
        Recorder,
        Recorder,
        Recorder,
        Recorder,
        Recorder,
        Recorder,
        Recorder,
        Recorder,
        Recorder,
        Recorder,
    > {
        CompositeJobHandler::new(
            Recorder::new("scan", Arc::clone(calls)),
            Recorder::new("reindex", Arc::clone(calls)),
            Recorder::new("artwork", Arc::clone(calls)),
            Recorder::new("trickplay", Arc::clone(calls)),
            Recorder::new("ingest", Arc::clone(calls)),
            Recorder::new("metadata", Arc::clone(calls)),
            Recorder::new("relink", Arc::clone(calls)),
            Recorder::new("subtitles", Arc::clone(calls)),
            Recorder::new("transcription", Arc::clone(calls)),
            Recorder::new("translation", Arc::clone(calls)),
            Recorder::new("upscale", Arc::clone(calls)),
            Recorder::new("combine", Arc::clone(calls)),
            Recorder::new("fetch", Arc::clone(calls)),
            Recorder::new("eviction", Arc::clone(calls)),
            Recorder::new("nightly", Arc::clone(calls)),
            Recorder::new("retention", Arc::clone(calls)),
            Recorder::new("sweep", Arc::clone(calls)),
        )
    }

    #[tokio::test]
    async fn routes_library_scan_to_scan_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls).handle(&job(JobKind::LibraryScan)).await.unwrap();
        assert_eq!(*calls.lock().unwrap(), ["scan"]);
    }

    #[tokio::test]
    async fn routes_search_reindex_to_reindex_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls).handle(&job(JobKind::SearchReindex)).await.unwrap();
        assert_eq!(*calls.lock().unwrap(), ["reindex"]);
    }

    #[tokio::test]
    async fn routes_artwork_to_artwork_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls).handle(&job(JobKind::Artwork)).await.unwrap();
        assert_eq!(*calls.lock().unwrap(), ["artwork"]);
    }

    #[tokio::test]
    async fn routes_trickplay_to_trickplay_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls).handle(&job(JobKind::Trickplay)).await.unwrap();
        assert_eq!(*calls.lock().unwrap(), ["trickplay"]);
    }

    #[tokio::test]
    async fn routes_ingest_to_ingest_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls).handle(&job(JobKind::Ingest)).await.unwrap();
        assert_eq!(*calls.lock().unwrap(), ["ingest"]);
    }

    #[tokio::test]
    async fn routes_metadata_to_metadata_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls).handle(&job(JobKind::Metadata)).await.unwrap();
        assert_eq!(*calls.lock().unwrap(), ["metadata"]);
    }

    #[tokio::test]
    async fn routes_relink_to_relink_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls).handle(&job(JobKind::Relink)).await.unwrap();
        assert_eq!(*calls.lock().unwrap(), ["relink"]);
    }

    #[tokio::test]
    async fn routes_subtitles_to_subtitles_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls).handle(&job(JobKind::Subtitles)).await.unwrap();
        assert_eq!(*calls.lock().unwrap(), ["subtitles"]);
    }

    #[tokio::test]
    async fn routes_transcription_to_transcription_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls).handle(&job(JobKind::Transcription)).await.unwrap();
        assert_eq!(*calls.lock().unwrap(), ["transcription"]);
    }

    #[tokio::test]
    async fn routes_translation_to_translation_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls).handle(&job(JobKind::Translation)).await.unwrap();
        assert_eq!(*calls.lock().unwrap(), ["translation"]);
    }

    #[tokio::test]
    async fn routes_upscale_to_upscale_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls).handle(&job(JobKind::Upscale)).await.unwrap();
        assert_eq!(*calls.lock().unwrap(), ["upscale"]);
    }

    #[tokio::test]
    async fn routes_combine_to_combine_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls).handle(&job(JobKind::Combine)).await.unwrap();
        assert_eq!(*calls.lock().unwrap(), ["combine"]);
    }

    #[tokio::test]
    async fn routes_fetch_to_fetch_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls).handle(&job(JobKind::Fetch)).await.unwrap();
        assert_eq!(*calls.lock().unwrap(), ["fetch"]);
    }

    #[tokio::test]
    async fn routes_cache_eviction_to_eviction_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls).handle(&job(JobKind::CacheEviction)).await.unwrap();
        assert_eq!(*calls.lock().unwrap(), ["eviction"]);
    }

    #[tokio::test]
    async fn routes_scheduled_scan_to_nightly_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls).handle(&job(JobKind::ScheduledScan)).await.unwrap();
        assert_eq!(*calls.lock().unwrap(), ["nightly"]);
    }

    #[tokio::test]
    async fn routes_retention_to_retention_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls).handle(&job(JobKind::Retention)).await.unwrap();
        assert_eq!(*calls.lock().unwrap(), ["retention"]);
    }

    #[tokio::test]
    async fn routes_orphan_sweep_to_sweep_handler() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        composite(&calls).handle(&job(JobKind::OrphanSweep)).await.unwrap();
        assert_eq!(*calls.lock().unwrap(), ["sweep"]);
    }

    #[tokio::test]
    async fn unhandled_kind_is_permanent_and_routes_nowhere() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let error = composite(&calls).handle(&job(JobKind::Fingerprint)).await.unwrap_err();
        assert!(matches!(error, JobError::Permanent(_)));
        assert!(calls.lock().unwrap().is_empty());
    }
}

use domain::catalog::{Version, VersionId};
use domain::error::UpscaleError;
use domain::job::{Job, JobKind, JobPriority};
use domain::media::{MediaProbe, UpscaleProvider, UpscaleSpec};
use domain::repository::{CatalogRepository, JobRepository};
use jiff::Timestamp;
use services::job::queued_job;
use services::library::{
    PlaybackPrepJobPayload, UpscaleJobPayload, container_of, derive_id, quality_from_height,
    upscaled_output_path,
};
use uuid::Uuid;

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct UpscaleJobHandler<P, C, M, J> {
    provider: P,
    catalog: C,
    probe: M,
    jobs: J,
}

impl<P, C, M, J> UpscaleJobHandler<P, C, M, J> {
    pub fn new(provider: P, catalog: C, probe: M, jobs: J) -> Self {
        Self { provider, catalog, probe, jobs }
    }
}

impl<P, C, M, J> JobHandler for UpscaleJobHandler<P, C, M, J>
where
    P: UpscaleProvider + Send + Sync,
    C: CatalogRepository + Send + Sync,
    M: MediaProbe + Send + Sync,
    J: JobRepository + Send + Sync,
{
    async fn handle(&self, job: &Job) -> Result<(), JobError> {
        let payload = UpscaleJobPayload::decode(&job.payload)
            .map_err(|e| JobError::Permanent(format!("invalid upscale payload: {e}")))?;

        let Some(detail) = self
            .catalog
            .version_detail(&payload.version_id)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?
        else {
            tracing::debug!("version gone; skipping upscale");
            return Ok(());
        };

        let target_quality = quality_from_height(payload.target_height);
        let output_path =
            upscaled_output_path(&detail.version.path, target_quality, &Uuid::new_v4().to_string());
        let request = UpscaleSpec {
            source_path: detail.version.path.clone(),
            target_height: payload.target_height,
            output_path,
        };
        tracing::info!(
            "upscaling version [{}] to {}p",
            payload.version_id.0,
            payload.target_height
        );
        let output = match self.provider.upscale(&request).await {
            Ok(output) => output,
            Err(UpscaleError::Unsupported(reason)) => {
                tracing::debug!("upscale unsupported: {reason}");
                return Ok(());
            }
            Err(UpscaleError::Precondition(reason)) => {
                return Err(JobError::Permanent(format!("upscale precondition: {reason}")));
            }
            Err(UpscaleError::Backend(reason)) => return Err(JobError::Retryable(reason)),
        };

        let probe =
            self.probe.probe(&output.path).await.map_err(|e| JobError::Retryable(e.to_string()))?;

        let now = Timestamp::now();
        let version_id = VersionId(derive_id("version", &output.path));
        let version = Version {
            id: version_id.clone(),
            title: detail.version.title.clone(),
            library: detail.version.library.clone(),
            quality: quality_from_height(payload.target_height),
            container: container_of(&output.path),
            path: output.path.clone(),
            size_bytes: output.size_bytes,
            duration_ms: probe.duration_ms,
            available: true,
            added_at: now,
            updated_at: now,
        };
        self.catalog
            .upsert_version(version)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?;
        self.catalog
            .set_version_tracks(
                &version_id,
                &probe.video,
                &probe.audio,
                &probe.subtitles,
                &probe.chapters,
            )
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?;
        let prep = PlaybackPrepJobPayload { version_id: version_id.clone() }.encode();
        let queued = queued_job(JobKind::PlaybackPrep, JobPriority::High, prep, Some(&job.id), now);
        if let Err(err) = self.jobs.enqueue(queued).await {
            #[rustfmt::skip]
            tracing::warn!("upscaled version [{}] was not queued for playback prep, so it stays unprepared until its files are re-read: {err}", version_id.0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use domain::catalog::{MovieId, TitleId, Version, VersionId};
    use domain::common::Quality;
    use domain::error::ProbeError;
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use domain::media::{ProbeResult, UpscaleOutput, VideoTrack};
    use jiff::Timestamp;
    use mocks::{MockCatalogRepo, MockJobStore};
    use services::library::{UpscaleJobPayload, derive_id};

    use super::*;

    enum Mode {
        Ok,
        Unsupported,
        Precondition,
        Backend,
    }

    struct MockUpscaleProvider {
        mode: Mode,
        captured: Arc<Mutex<Option<String>>>,
    }

    impl UpscaleProvider for MockUpscaleProvider {
        async fn upscale(&self, request: &UpscaleSpec) -> Result<UpscaleOutput, UpscaleError> {
            *self.captured.lock().unwrap() = Some(request.output_path.clone());
            match self.mode {
                Mode::Ok => {
                    Ok(UpscaleOutput { path: request.output_path.clone(), size_bytes: 4242 })
                }
                Mode::Unsupported => Err(UpscaleError::Unsupported("already large".into())),
                Mode::Precondition => Err(UpscaleError::Precondition("read only".into())),
                Mode::Backend => Err(UpscaleError::Backend("boom".into())),
            }
        }
    }

    #[derive(Default)]
    struct MockProbe {
        fail: bool,
        fail_catalog_after: Option<MockCatalogRepo>,
    }

    impl MediaProbe for MockProbe {
        async fn probe(&self, _path: &str) -> Result<ProbeResult, ProbeError> {
            if self.fail {
                return Err(ProbeError::Backend("probe boom".into()));
            }
            if let Some(catalog) = &self.fail_catalog_after {
                catalog.set_fail();
            }
            Ok(ProbeResult {
                duration_ms: 2000,
                video: vec![VideoTrack {
                    index: 0,
                    codec: "h264".into(),
                    width: 1920,
                    height: 1080,
                    bit_depth: 8,
                    hdr: None,
                    frame_rate: 24.0,
                    bitrate: None,
                }],
                audio: Vec::new(),
                subtitles: Vec::new(),
                chapters: Vec::new(),
            })
        }
    }

    fn provider(mode: Mode) -> (MockUpscaleProvider, Arc<Mutex<Option<String>>>) {
        let captured = Arc::new(Mutex::new(None));
        (MockUpscaleProvider { mode, captured: captured.clone() }, captured)
    }

    fn source_version() -> Version {
        Version {
            id: VersionId("v1".into()),
            title: TitleId::Movie(MovieId("m1".into())),
            library: domain::library::LibraryId("lib".into()),
            quality: Quality::Sd,
            container: "mkv".into(),
            path: "/m/Movie (2011)/Movie (2011) 480p.mkv".into(),
            size_bytes: 100,
            duration_ms: 2000,
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn job(raw: String) -> Job {
        let now = Timestamp::UNIX_EPOCH;
        Job {
            id: JobId("job-1".into()),
            kind: JobKind::Upscale,
            status: JobStatus::Running,
            priority: JobPriority::Low,
            payload: raw,
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

    fn payload() -> String {
        UpscaleJobPayload { version_id: VersionId("v1".into()), target_height: 1080 }.encode()
    }

    #[tokio::test]
    async fn creates_a_new_upscaled_version_next_to_source() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(source_version());
        let (prov, captured) = provider(Mode::Ok);
        let jobs = MockJobStore::new();
        let handler =
            UpscaleJobHandler::new(prov, catalog.clone(), MockProbe::default(), jobs.clone());

        handler.handle(&job(payload())).await.unwrap();

        let out_path = captured.lock().unwrap().clone().unwrap();
        assert!(out_path.starts_with("/m/Movie (2011)/"));
        assert!(out_path.contains("1080p [Upscaled "));
        assert!(out_path.ends_with(".mp4"));

        let new_id = VersionId(derive_id("version", &out_path));
        let detail = catalog.version_detail(&new_id).await.unwrap().unwrap();
        assert_eq!(detail.version.quality, Quality::Fhd);
        assert_eq!(detail.version.size_bytes, 4242);
        assert_eq!(detail.version.container, "mp4");
        assert_eq!(detail.version.title, TitleId::Movie(MovieId("m1".into())));
        assert!(detail.version.available);
        assert_eq!(detail.video.len(), 1);

        let queued = jobs.list().await.unwrap();
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].kind, JobKind::PlaybackPrep);
        assert_eq!(queued[0].priority, JobPriority::High);
        assert_eq!(queued[0].status, JobStatus::Queued);
        assert_eq!(queued[0].parent_id, Some(JobId("job-1".into())));
        assert_eq!(PlaybackPrepJobPayload::decode(&queued[0].payload).unwrap().version_id, new_id);
    }

    #[tokio::test]
    async fn an_upscale_that_cannot_queue_its_prep_still_succeeds() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(source_version());
        let (prov, captured) = provider(Mode::Ok);
        let jobs = MockJobStore::new();
        jobs.set_fail();
        let handler = UpscaleJobHandler::new(prov, catalog.clone(), MockProbe::default(), jobs);

        handler.handle(&job(payload())).await.unwrap();

        let out_path = captured.lock().unwrap().clone().unwrap();
        let new_id = VersionId(derive_id("version", &out_path));
        assert!(catalog.version_detail(&new_id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn a_version_that_is_gone_is_a_no_op() {
        let (prov, captured) = provider(Mode::Ok);
        let handler = UpscaleJobHandler::new(
            prov,
            MockCatalogRepo::new(),
            MockProbe::default(),
            MockJobStore::new(),
        );

        handler.handle(&job(payload())).await.unwrap();

        assert!(captured.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn unsupported_is_a_no_op() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(source_version());
        let (prov, captured) = provider(Mode::Unsupported);
        let jobs = MockJobStore::new();
        let handler =
            UpscaleJobHandler::new(prov, catalog.clone(), MockProbe::default(), jobs.clone());

        handler.handle(&job(payload())).await.unwrap();

        let out_path = captured.lock().unwrap().clone().unwrap();
        let new_id = VersionId(derive_id("version", &out_path));
        assert!(catalog.version_detail(&new_id).await.unwrap().is_none());
        assert!(jobs.list().await.unwrap().is_empty());
    }

    async fn fails_without_queueing(
        mode: Mode,
        probe: MockProbe,
        catalog: MockCatalogRepo,
    ) -> JobError {
        let (prov, _) = provider(mode);
        let jobs = MockJobStore::new();
        let handler = UpscaleJobHandler::new(prov, catalog, probe, jobs.clone());
        let err = handler.handle(&job(payload())).await.unwrap_err();
        assert!(jobs.list().await.unwrap().is_empty());
        err
    }

    fn with_source() -> MockCatalogRepo {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(source_version());
        catalog
    }

    #[tokio::test]
    async fn precondition_is_permanent() {
        let err =
            fails_without_queueing(Mode::Precondition, MockProbe::default(), with_source()).await;
        assert!(matches!(err, JobError::Permanent(_)));
    }

    #[tokio::test]
    async fn backend_failure_is_retryable() {
        let err = fails_without_queueing(Mode::Backend, MockProbe::default(), with_source()).await;
        assert!(matches!(err, JobError::Retryable(_)));
    }

    #[tokio::test]
    async fn probe_failure_is_retryable() {
        let probe = MockProbe { fail: true, ..MockProbe::default() };
        let err = fails_without_queueing(Mode::Ok, probe, with_source()).await;
        assert!(matches!(err, JobError::Retryable(_)));
    }

    #[tokio::test]
    async fn version_detail_failure_is_retryable() {
        let catalog = with_source();
        catalog.set_fail();
        let err = fails_without_queueing(Mode::Ok, MockProbe::default(), catalog).await;
        assert!(matches!(err, JobError::Retryable(_)));
    }

    #[tokio::test]
    async fn persist_failure_is_retryable() {
        let catalog = with_source();
        let probe = MockProbe { fail_catalog_after: Some(catalog.clone()), ..MockProbe::default() };
        let err = fails_without_queueing(Mode::Ok, probe, catalog).await;
        assert!(matches!(err, JobError::Retryable(_)));
    }

    #[tokio::test]
    async fn invalid_payload_is_permanent() {
        let (prov, _) = provider(Mode::Ok);
        let handler = UpscaleJobHandler::new(
            prov,
            MockCatalogRepo::new(),
            MockProbe::default(),
            MockJobStore::new(),
        );
        assert!(matches!(
            handler.handle(&job("garbage".into())).await.unwrap_err(),
            JobError::Permanent(_)
        ));
    }
}

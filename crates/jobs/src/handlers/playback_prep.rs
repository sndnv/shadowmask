use domain::job::Job;
use domain::media::PlaybackPreparer;
use domain::repository::CatalogRepository;
use services::library::PlaybackPrepJobPayload;

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct PlaybackPrepJobHandler<C, P> {
    catalog: C,
    preparer: P,
}

impl<C, P> PlaybackPrepJobHandler<C, P> {
    pub fn new(catalog: C, preparer: P) -> Self {
        Self { catalog, preparer }
    }
}

impl<C, P> JobHandler for PlaybackPrepJobHandler<C, P>
where
    C: CatalogRepository + Send + Sync,
    P: PlaybackPreparer + Send + Sync,
{
    async fn handle(&self, job: &Job) -> Result<(), JobError> {
        let payload = PlaybackPrepJobPayload::decode(&job.payload)
            .map_err(|e| JobError::Permanent(format!("invalid playback prep payload: {e}")))?;
        let version = &payload.version_id;
        let detail = self
            .catalog
            .version_detail(version)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?;
        let Some(detail) = detail else {
            tracing::info!("version [{}] is gone, so there is nothing to prepare", version.0);
            return Ok(());
        };
        #[rustfmt::skip]
        tracing::info!("preparing version [{}] for playback from [{}]", version.0, detail.version.path);
        let prep = self
            .preparer
            .prepare(version, &detail.version.path, &detail.subtitles)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?;
        self.catalog
            .set_playback_prep(version, &prep)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use domain::catalog::{MovieId, TitleId, Version, VersionDetail, VersionId};
    use domain::common::Quality;
    use domain::error::PlaybackPrepError;
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use domain::media::{DetectedMarkers, EmbeddedSubtitleTrack, PlaybackPrep, SubtitleFormat};
    use jiff::Timestamp;
    use mocks::MockCatalogRepo;

    use super::*;

    type Prepared = (String, Vec<u32>);

    #[derive(Clone, Default)]
    struct MockPreparer {
        fail: bool,
        seen: Arc<Mutex<Vec<Prepared>>>,
    }

    impl PlaybackPreparer for MockPreparer {
        async fn prepare(
            &self,
            _version: &VersionId,
            input_path: &str,
            subtitles: &[EmbeddedSubtitleTrack],
        ) -> Result<PlaybackPrep, PlaybackPrepError> {
            let indices: Vec<u32> = subtitles.iter().map(|track| track.index).collect();
            self.seen.lock().unwrap().push((input_path.to_owned(), indices.clone()));
            if self.fail {
                return Err(PlaybackPrepError::Backend("ffmpeg".into()));
            }
            Ok(PlaybackPrep {
                keyframes_us: vec![0, 4_000_000],
                extracted_subtitles: indices,
                prepared_with: "9.0.2".into(),
            })
        }
    }

    fn detail() -> VersionDetail {
        VersionDetail {
            version: Version {
                id: VersionId("v1".into()),
                title: TitleId::Movie(MovieId("m1".into())),
                library: domain::library::LibraryId("lib".into()),
                quality: Quality::Hd,
                container: "mkv".into(),
                path: "/m/v1.mkv".into(),
                size_bytes: 1,
                duration_ms: 1000,
                available: true,
                added_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
            },
            video: Vec::new(),
            audio: Vec::new(),
            subtitles: vec![EmbeddedSubtitleTrack {
                index: 2,
                language: None,
                format: SubtitleFormat::Srt,
                forced: false,
                default: false,
            }],
            subtitle_files: Vec::new(),
            chapters: Vec::new(),
            markers: DetectedMarkers::default(),
            trickplay: Vec::new(),
            extracted_subtitles: Vec::new(),
        }
    }

    fn job(raw: String) -> Job {
        let now = Timestamp::UNIX_EPOCH;
        Job {
            id: JobId("job-1".into()),
            kind: JobKind::PlaybackPrep,
            status: JobStatus::Running,
            priority: JobPriority::High,
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
        PlaybackPrepJobPayload { version_id: VersionId("v1".into()) }.encode()
    }

    #[tokio::test]
    async fn prepares_the_version_from_its_catalog_entry_and_stores_the_result() {
        let catalog = MockCatalogRepo::new();
        catalog.seed_version_detail(detail());
        let preparer = MockPreparer::default();
        let handler = PlaybackPrepJobHandler::new(catalog.clone(), preparer.clone());

        handler.handle(&job(payload())).await.unwrap();

        assert_eq!(*preparer.seen.lock().unwrap(), vec![("/m/v1.mkv".to_owned(), vec![2])]);
        let version = VersionId("v1".into());
        assert_eq!(catalog.keyframes_us(&version).await.unwrap(), Some(vec![0, 4_000_000]));
        let states = catalog.playback_prep_states().await.unwrap();
        let state = states.iter().find(|state| state.version == version).unwrap();
        assert_eq!(state.prepared_with.as_deref(), Some("9.0.2"));
        let stored = catalog.version_detail(&version).await.unwrap().unwrap();
        assert_eq!(stored.extracted_subtitles, vec![2]);
    }

    #[tokio::test]
    async fn a_version_that_is_gone_is_not_prepared() {
        let preparer = MockPreparer::default();
        let handler = PlaybackPrepJobHandler::new(MockCatalogRepo::new(), preparer.clone());

        handler.handle(&job(payload())).await.unwrap();

        assert!(preparer.seen.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_failed_preparation_is_retryable() {
        let catalog = MockCatalogRepo::new();
        catalog.seed_version_detail(detail());
        let handler =
            PlaybackPrepJobHandler::new(catalog, MockPreparer { fail: true, ..Default::default() });

        assert!(matches!(
            handler.handle(&job(payload())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn a_catalog_that_is_down_is_retryable() {
        let catalog = MockCatalogRepo::new();
        catalog.set_fail();
        let handler = PlaybackPrepJobHandler::new(catalog, MockPreparer::default());

        assert!(matches!(
            handler.handle(&job(payload())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn a_result_that_cannot_be_stored_is_retryable() {
        let catalog = MockCatalogRepo::new();
        catalog.seed_version_detail(detail());
        catalog.set_fail_writes();
        let handler = PlaybackPrepJobHandler::new(catalog, MockPreparer::default());

        assert!(matches!(
            handler.handle(&job(payload())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn an_invalid_payload_is_permanent() {
        let handler = PlaybackPrepJobHandler::new(MockCatalogRepo::new(), MockPreparer::default());

        assert!(matches!(
            handler.handle(&job("garbage".into())).await.unwrap_err(),
            JobError::Permanent(_)
        ));
    }
}

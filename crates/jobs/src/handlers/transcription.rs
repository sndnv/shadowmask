use domain::error::TranscriptionError;
use domain::job::{Job, TranslationTrigger};
use domain::media::{
    SubtitleFile, SubtitleFileId, SubtitleSource, SubtitleStore, TranscriptionProvider,
    TranscriptionSpec,
};
use domain::repository::CatalogRepository;
use services::library::TranscriptionJobPayload;

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct TranscriptionJobHandler<P, C, S, T> {
    provider: P,
    catalog: C,
    store: S,
    trigger: T,
}

impl<P, C, S, T> TranscriptionJobHandler<P, C, S, T> {
    pub fn new(provider: P, catalog: C, store: S, trigger: T) -> Self {
        Self { provider, catalog, store, trigger }
    }
}

impl<P, C, S, T> JobHandler for TranscriptionJobHandler<P, C, S, T>
where
    P: TranscriptionProvider + Send + Sync,
    C: CatalogRepository + Send + Sync,
    S: SubtitleStore + Send + Sync,
    T: TranslationTrigger + Send + Sync,
{
    async fn handle(&self, job: &Job) -> Result<(), JobError> {
        let payload = TranscriptionJobPayload::decode(&job.payload)
            .map_err(|e| JobError::Permanent(format!("invalid transcription payload: {e}")))?;

        let Some(detail) = self
            .catalog
            .version_detail(&payload.version_id)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?
        else {
            tracing::debug!("version gone; skipping transcription");
            return Ok(());
        };
        if !payload.force && !detail.subtitle_files.is_empty() {
            tracing::debug!("subtitle already present; skipping transcription");
            return Ok(());
        }

        let request = TranscriptionSpec {
            audio_path: payload.source_path.clone(),
            source_language: None,
            audio_track_index: payload.audio_track_index,
        };
        let track = payload
            .audio_track_index
            .map(|index| index.to_string())
            .unwrap_or_else(|| "default".into());
        let container_language = payload
            .source_language
            .as_deref()
            .map(str::trim)
            .filter(|code| !code.is_empty() && *code != "und")
            .unwrap_or("unknown");
        tracing::info!(
            "transcribing version [{}] from [{}] (audio track {}, container language {})",
            payload.version_id.0,
            payload.source_path,
            track,
            container_language
        );
        let subtitle = match self.provider.transcribe(&request).await {
            Ok(subtitle) => {
                tracing::info!(
                    "transcription produced a [{:?}] subtitle for version [{}]",
                    subtitle.format,
                    payload.version_id.0
                );
                subtitle
            }
            Err(TranscriptionError::Unsupported(reason)) => {
                tracing::debug!("transcription unsupported: {reason}");
                return Ok(());
            }
            Err(e) => return Err(JobError::Retryable(e.to_string())),
        };

        if !subtitle.has_text() {
            tracing::info!(
                "transcription produced no usable text for version [{}]; skipping",
                payload.version_id.0
            );
            return Ok(());
        }

        let path = self
            .store
            .store(&payload.version_id, "generated", subtitle.format, &subtitle.content)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?;
        let file = SubtitleFile {
            id: SubtitleFileId(format!("generated:{}", payload.version_id.0)),
            version: payload.version_id.clone(),
            language: None,
            format: subtitle.format,
            source: SubtitleSource::Generated,
            path,
            translated_from: None,
            label: None,
            pinned: false,
        };
        let current = self
            .catalog
            .version_detail(&payload.version_id)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?;
        let mut merged: Vec<SubtitleFile> = current
            .map(|detail| detail.subtitle_files)
            .unwrap_or_default()
            .into_iter()
            .filter(|existing| existing.source != SubtitleSource::Generated)
            .collect();
        merged.push(file);
        self.catalog
            .set_subtitle_files(&payload.version_id, &merged)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?;
        self.trigger
            .trigger(&payload.version_id, Some(&job.id))
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use domain::catalog::{TitleId, Version, VersionId};
    use domain::common::Quality;
    use domain::error::RepositoryError;
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use domain::media::{FetchedSubtitle, SubtitleFile, SubtitleFileId, SubtitleFormat};
    use jiff::Timestamp;
    use mocks::MockCatalogRepo;
    use services::library::TranscriptionJobPayload;

    use super::*;

    enum ProviderMode {
        Ok,
        Empty,
        Unsupported,
        Backend,
    }

    struct MockProvider {
        mode: ProviderMode,
    }

    impl TranscriptionProvider for MockProvider {
        async fn transcribe(
            &self,
            _request: &TranscriptionSpec,
        ) -> Result<FetchedSubtitle, TranscriptionError> {
            match &self.mode {
                ProviderMode::Ok => Ok(FetchedSubtitle {
                    content: "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nhi\n".into(),
                    format: SubtitleFormat::Vtt,
                }),
                ProviderMode::Empty => {
                    Ok(FetchedSubtitle { content: "WEBVTT\n".into(), format: SubtitleFormat::Vtt })
                }
                ProviderMode::Unsupported => {
                    Err(TranscriptionError::Unsupported("no engine".into()))
                }
                ProviderMode::Backend => Err(TranscriptionError::Backend("boom".into())),
            }
        }
    }

    #[derive(Default)]
    struct MockStore {
        fail: bool,
        stored: Arc<Mutex<Vec<String>>>,
        fail_catalog_after: Option<MockCatalogRepo>,
    }

    impl SubtitleStore for MockStore {
        async fn store(
            &self,
            version: &VersionId,
            file_id: &str,
            format: SubtitleFormat,
            _content: &str,
        ) -> Result<String, domain::error::SubtitleError> {
            if self.fail {
                return Err(domain::error::SubtitleError::Store("disk full".into()));
            }
            let path = format!("/subs/{}/{file_id}.{}", version.0, format.extension());
            self.stored.lock().unwrap().push(path.clone());
            if let Some(catalog) = &self.fail_catalog_after {
                catalog.set_fail();
            }
            Ok(path)
        }
    }

    #[derive(Clone, Default)]
    struct MockTrigger {
        fail: bool,
    }

    impl TranslationTrigger for MockTrigger {
        async fn trigger(
            &self,
            _version_id: &VersionId,
            _parent: Option<&JobId>,
        ) -> Result<(), RepositoryError> {
            if self.fail { Err(RepositoryError::Backend("trigger boom".into())) } else { Ok(()) }
        }
    }

    fn version() -> Version {
        Version {
            id: VersionId("v1".into()),
            title: TitleId::Movie(domain::catalog::MovieId("m1".into())),
            library: domain::library::LibraryId("lib".into()),
            quality: Quality::Hd,
            container: "mkv".into(),
            path: "/m/v1.mkv".into(),
            size_bytes: 1,
            duration_ms: 1000,
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn job(raw: String) -> Job {
        let now = Timestamp::UNIX_EPOCH;
        Job {
            id: JobId("job-1".into()),
            kind: JobKind::Transcription,
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
        payload_with_force(false)
    }

    fn payload_with_force(force: bool) -> String {
        TranscriptionJobPayload {
            version_id: VersionId("v1".into()),
            source_path: "/m/v1.mkv".into(),
            source_language: None,
            audio_track_index: None,
            force,
        }
        .encode()
    }

    fn subtitle(id: &str, source: SubtitleSource) -> SubtitleFile {
        SubtitleFile {
            id: SubtitleFileId(id.into()),
            version: VersionId("v1".into()),
            language: None,
            format: SubtitleFormat::Srt,
            source,
            path: format!("/m/{id}.srt"),
            translated_from: None,
            label: None,
            pinned: false,
        }
    }

    #[tokio::test]
    async fn generates_soft_subtitle_when_no_existing() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version());
        let store = MockStore::default();
        let handler = TranscriptionJobHandler::new(
            MockProvider { mode: ProviderMode::Ok },
            catalog.clone(),
            MockStore { stored: store.stored.clone(), ..MockStore::default() },
            MockTrigger::default(),
        );

        handler.handle(&job(payload())).await.unwrap();

        assert_eq!(store.stored.lock().unwrap().len(), 1);
        let detail = catalog.version_detail(&VersionId("v1".into())).await.unwrap().unwrap();
        assert_eq!(detail.subtitle_files.len(), 1);
        assert_eq!(detail.subtitle_files[0].source, SubtitleSource::Generated);
        assert_eq!(detail.subtitle_files[0].format, SubtitleFormat::Vtt);
    }

    #[tokio::test]
    async fn empty_transcription_is_not_stored() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version());
        let store = MockStore::default();
        let handler = TranscriptionJobHandler::new(
            MockProvider { mode: ProviderMode::Empty },
            catalog.clone(),
            MockStore { stored: store.stored.clone(), ..MockStore::default() },
            MockTrigger::default(),
        );

        handler.handle(&job(payload())).await.unwrap();

        assert!(store.stored.lock().unwrap().is_empty());
        let detail = catalog.version_detail(&VersionId("v1".into())).await.unwrap().unwrap();
        assert!(detail.subtitle_files.is_empty());
    }

    #[tokio::test]
    async fn no_op_when_subtitle_already_present() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version());
        catalog
            .set_subtitle_files(
                &VersionId("v1".into()),
                &[SubtitleFile {
                    id: SubtitleFileId("sidecar".into()),
                    version: VersionId("v1".into()),
                    language: None,
                    format: SubtitleFormat::Srt,
                    source: SubtitleSource::External,
                    path: "/m/v1.srt".into(),
                    translated_from: None,
                    label: None,
                    pinned: false,
                }],
            )
            .await
            .unwrap();
        let store = MockStore::default();
        let handler = TranscriptionJobHandler::new(
            MockProvider { mode: ProviderMode::Ok },
            catalog.clone(),
            MockStore { stored: store.stored.clone(), ..MockStore::default() },
            MockTrigger::default(),
        );

        handler.handle(&job(payload())).await.unwrap();

        assert!(store.stored.lock().unwrap().is_empty());
        let detail = catalog.version_detail(&VersionId("v1".into())).await.unwrap().unwrap();
        assert_eq!(detail.subtitle_files.len(), 1);
        assert_eq!(detail.subtitle_files[0].source, SubtitleSource::External);
    }

    #[tokio::test]
    async fn a_forced_run_transcribes_over_an_existing_subtitle() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version());
        catalog
            .set_subtitle_files(
                &VersionId("v1".into()),
                &[subtitle("opensubtitles:v1:1", SubtitleSource::OpenSubtitles)],
            )
            .await
            .unwrap();
        let store = MockStore::default();
        let handler = TranscriptionJobHandler::new(
            MockProvider { mode: ProviderMode::Ok },
            catalog.clone(),
            MockStore { stored: store.stored.clone(), ..MockStore::default() },
            MockTrigger::default(),
        );

        handler.handle(&job(payload_with_force(true))).await.unwrap();

        assert_eq!(store.stored.lock().unwrap().len(), 1);
        let detail = catalog.version_detail(&VersionId("v1".into())).await.unwrap().unwrap();
        let sources: Vec<SubtitleSource> = detail.subtitle_files.iter().map(|f| f.source).collect();
        assert!(
            sources.contains(&SubtitleSource::Generated),
            "the admin asked for this explicitly, so it must run"
        );
        assert!(
            sources.contains(&SubtitleSource::OpenSubtitles),
            "forcing must add a track, never wipe the ones already fetched"
        );
    }

    #[tokio::test]
    async fn a_forced_rerun_replaces_the_previous_generated_subtitle() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version());
        catalog
            .set_subtitle_files(
                &VersionId("v1".into()),
                &[
                    subtitle("external", SubtitleSource::External),
                    subtitle("generated:v1", SubtitleSource::Generated),
                ],
            )
            .await
            .unwrap();
        let handler = TranscriptionJobHandler::new(
            MockProvider { mode: ProviderMode::Ok },
            catalog.clone(),
            MockStore::default(),
            MockTrigger::default(),
        );

        handler.handle(&job(payload_with_force(true))).await.unwrap();

        let detail = catalog.version_detail(&VersionId("v1".into())).await.unwrap().unwrap();
        let generated: Vec<&SubtitleFile> = detail
            .subtitle_files
            .iter()
            .filter(|f| f.source == SubtitleSource::Generated)
            .collect();
        assert_eq!(generated.len(), 1, "re-running must not stack up a second generated track");
        assert_eq!(
            generated[0].format,
            SubtitleFormat::Vtt,
            "the seeded track was Srt, so Vtt proves the rerun replaced it"
        );
        assert_eq!(detail.subtitle_files.len(), 2);
    }

    #[tokio::test]
    async fn no_op_when_version_missing() {
        let store = MockStore::default();
        let handler = TranscriptionJobHandler::new(
            MockProvider { mode: ProviderMode::Ok },
            MockCatalogRepo::new(),
            MockStore { stored: store.stored.clone(), ..MockStore::default() },
            MockTrigger::default(),
        );

        handler.handle(&job(payload())).await.unwrap();

        assert!(store.stored.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn unsupported_is_a_no_op() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version());
        let store = MockStore::default();
        let handler = TranscriptionJobHandler::new(
            MockProvider { mode: ProviderMode::Unsupported },
            catalog.clone(),
            MockStore { stored: store.stored.clone(), ..MockStore::default() },
            MockTrigger::default(),
        );

        handler.handle(&job(payload())).await.unwrap();

        assert!(store.stored.lock().unwrap().is_empty());
        let detail = catalog.version_detail(&VersionId("v1".into())).await.unwrap().unwrap();
        assert!(detail.subtitle_files.is_empty());
    }

    #[tokio::test]
    async fn backend_failure_is_retryable() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version());
        let handler = TranscriptionJobHandler::new(
            MockProvider { mode: ProviderMode::Backend },
            catalog,
            MockStore::default(),
            MockTrigger::default(),
        );
        assert!(matches!(
            handler.handle(&job(payload())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn store_failure_is_retryable() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version());
        let handler = TranscriptionJobHandler::new(
            MockProvider { mode: ProviderMode::Ok },
            catalog,
            MockStore { fail: true, ..MockStore::default() },
            MockTrigger::default(),
        );
        assert!(matches!(
            handler.handle(&job(payload())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn catalog_read_failure_is_retryable() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version());
        catalog.set_fail();
        let handler = TranscriptionJobHandler::new(
            MockProvider { mode: ProviderMode::Ok },
            catalog,
            MockStore::default(),
            MockTrigger::default(),
        );
        assert!(matches!(
            handler.handle(&job(payload())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn catalog_write_failure_is_retryable() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version());
        let handler = TranscriptionJobHandler::new(
            MockProvider { mode: ProviderMode::Ok },
            catalog.clone(),
            MockStore { fail_catalog_after: Some(catalog), ..MockStore::default() },
            MockTrigger::default(),
        );
        assert!(matches!(
            handler.handle(&job(payload())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[derive(Clone, Default)]
    struct CapturingProvider {
        seen: Arc<Mutex<Option<Option<u32>>>>,
        seen_language: Arc<Mutex<Option<Option<String>>>>,
    }

    impl TranscriptionProvider for CapturingProvider {
        async fn transcribe(
            &self,
            request: &TranscriptionSpec,
        ) -> Result<FetchedSubtitle, TranscriptionError> {
            *self.seen.lock().unwrap() = Some(request.audio_track_index);
            *self.seen_language.lock().unwrap() =
                Some(request.source_language.as_ref().map(|code| code.0.clone()));
            Ok(FetchedSubtitle {
                content: "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nhi\n".into(),
                format: SubtitleFormat::Vtt,
            })
        }
    }

    #[tokio::test]
    async fn threads_audio_track_index_to_the_provider() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version());
        let provider = CapturingProvider::default();
        let handler = TranscriptionJobHandler::new(
            provider.clone(),
            catalog,
            MockStore::default(),
            MockTrigger::default(),
        );
        let raw = TranscriptionJobPayload {
            version_id: VersionId("v1".into()),
            source_path: "/m/v1.mkv".into(),
            source_language: None,
            audio_track_index: Some(3),
            force: false,
        }
        .encode();

        handler.handle(&job(raw)).await.unwrap();

        assert_eq!(*provider.seen.lock().unwrap(), Some(Some(3)));
    }

    #[tokio::test]
    async fn stores_unknown_language_and_auto_detects() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version());
        let provider = CapturingProvider::default();
        let handler = TranscriptionJobHandler::new(
            provider.clone(),
            catalog.clone(),
            MockStore::default(),
            MockTrigger::default(),
        );
        let raw = TranscriptionJobPayload {
            version_id: VersionId("v1".into()),
            source_path: "/m/v1.mkv".into(),
            source_language: Some("eng".into()),
            audio_track_index: None,
            force: false,
        }
        .encode();

        handler.handle(&job(raw)).await.unwrap();

        assert_eq!(*provider.seen_language.lock().unwrap(), Some(None));
        let detail = catalog.version_detail(&VersionId("v1".into())).await.unwrap().unwrap();
        assert_eq!(detail.subtitle_files[0].language, None);
    }

    #[tokio::test]
    async fn und_container_language_is_not_stored() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version());
        let handler = TranscriptionJobHandler::new(
            MockProvider { mode: ProviderMode::Ok },
            catalog.clone(),
            MockStore::default(),
            MockTrigger::default(),
        );
        let raw = TranscriptionJobPayload {
            version_id: VersionId("v1".into()),
            source_path: "/m/v1.mkv".into(),
            source_language: Some("und".into()),
            audio_track_index: None,
            force: false,
        }
        .encode();

        handler.handle(&job(raw)).await.unwrap();

        let detail = catalog.version_detail(&VersionId("v1".into())).await.unwrap().unwrap();
        assert_eq!(detail.subtitle_files[0].language, None);
    }

    #[tokio::test]
    async fn invalid_payload_is_permanent() {
        let handler = TranscriptionJobHandler::new(
            MockProvider { mode: ProviderMode::Ok },
            MockCatalogRepo::new(),
            MockStore::default(),
            MockTrigger::default(),
        );
        assert!(matches!(
            handler.handle(&job("garbage".into())).await.unwrap_err(),
            JobError::Permanent(_)
        ));
    }

    #[tokio::test]
    async fn translation_trigger_failure_is_retryable() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version());
        let handler = TranscriptionJobHandler::new(
            MockProvider { mode: ProviderMode::Ok },
            catalog,
            MockStore::default(),
            MockTrigger { fail: true },
        );
        assert!(matches!(
            handler.handle(&job(payload())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }
}

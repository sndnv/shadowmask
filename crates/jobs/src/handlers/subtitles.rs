use std::collections::HashSet;

use domain::common::LanguageCode;
use domain::error::SubtitleError;
use domain::job::{Job, JobId, TranscriptionTrigger, TranslationTrigger};
use domain::media::{
    SubtitleFile, SubtitleFileId, SubtitleProvider, SubtitleQuery, SubtitleSource, SubtitleStore,
    prune_orphaned_translations,
};
use domain::repository::CatalogRepository;
use services::library::SubtitleJobPayload;

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct SubtitlesJobHandler<P, C, S, T, T2> {
    provider: P,
    catalog: C,
    store: S,
    trigger: T,
    transcription: T2,
}

impl<P, C, S, T, T2> SubtitlesJobHandler<P, C, S, T, T2> {
    pub fn new(provider: P, catalog: C, store: S, trigger: T, transcription: T2) -> Self {
        Self { provider, catalog, store, trigger, transcription }
    }
}

impl<P, C, S, T, T2> SubtitlesJobHandler<P, C, S, T, T2>
where
    T2: TranscriptionTrigger + Send + Sync,
{
    async fn transcribe_on_miss(
        &self,
        payload: &SubtitleJobPayload,
        parent: &JobId,
    ) -> Result<(), JobError> {
        if payload.transcribe_on_miss {
            tracing::debug!(
                "no subtitles for version [{}]; falling back to transcription",
                payload.version_id.0
            );
            self.transcription
                .trigger(&payload.version_id, Some(parent))
                .await
                .map_err(|e| JobError::Retryable(e.to_string()))?;
        }
        Ok(())
    }
}

impl<P, C, S, T, T2> JobHandler for SubtitlesJobHandler<P, C, S, T, T2>
where
    P: SubtitleProvider + Send + Sync,
    C: CatalogRepository + Send + Sync,
    S: SubtitleStore + Send + Sync,
    T: TranslationTrigger + Send + Sync,
    T2: TranscriptionTrigger + Send + Sync,
{
    async fn handle(&self, job: &Job) -> Result<(), JobError> {
        let payload = SubtitleJobPayload::decode(&job.payload)
            .map_err(|e| JobError::Permanent(format!("invalid subtitle payload: {e}")))?;
        let languages: Vec<LanguageCode> =
            payload.languages.iter().map(|language| LanguageCode(language.clone())).collect();
        let query = SubtitleQuery {
            imdb_id: payload.imdb_id.clone(),
            query: payload.title.clone(),
            languages: languages.clone(),
            season: payload.season,
            episode: payload.episode,
        };
        tracing::info!("fetching subtitles for version [{}]", payload.version_id.0);
        let candidates = match self.provider.search(&query).await {
            Ok(candidates) => candidates,
            Err(SubtitleError::NotFound) => {
                tracing::debug!("no subtitles found");
                return self.transcribe_on_miss(&payload, &job.id).await;
            }
            Err(e) => return Err(JobError::Retryable(e.to_string())),
        };

        let mut fetched: Vec<SubtitleFile> = Vec::new();
        for language in &languages {
            let Some(candidate) =
                candidates.iter().find(|candidate| candidate.language.as_ref() == Some(language))
            else {
                continue;
            };
            let subtitle = self
                .provider
                .download(&candidate.file_id)
                .await
                .map_err(|e| JobError::Retryable(e.to_string()))?;
            let path = self
                .store
                .store(&payload.version_id, &candidate.file_id, subtitle.format, &subtitle.content)
                .await
                .map_err(|e| JobError::Retryable(e.to_string()))?;
            fetched.push(SubtitleFile {
                id: SubtitleFileId(format!(
                    "opensubtitles:{}:{}",
                    payload.version_id.0, candidate.file_id
                )),
                version: payload.version_id.clone(),
                language: candidate.language.clone(),
                format: subtitle.format,
                source: SubtitleSource::OpenSubtitles,
                path,
                translated_from: None,
                label: candidate.release_name.clone(),
                pinned: false,
            });
        }

        if fetched.is_empty() {
            return self.transcribe_on_miss(&payload, &job.id).await;
        }

        let existing = self
            .catalog
            .version_detail(&payload.version_id)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?;
        let mut merged: Vec<SubtitleFile> = existing
            .map(|detail| detail.subtitle_files)
            .unwrap_or_default()
            .into_iter()
            .filter(|file| file.source != SubtitleSource::OpenSubtitles || file.pinned)
            .collect();
        let kept: HashSet<SubtitleFileId> = merged.iter().map(|file| file.id.clone()).collect();
        merged.extend(fetched.into_iter().filter(|file| !kept.contains(&file.id)));
        let merged = prune_orphaned_translations(merged);
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
    use domain::error::{RepositoryError, SubtitleError};
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use domain::media::{
        FetchedSubtitle, SubtitleCandidate, SubtitleFile, SubtitleFileId, SubtitleFormat,
        SubtitleQuery, SubtitleSource,
    };
    use jiff::Timestamp;
    use mocks::MockCatalogRepo;
    use services::library::SubtitleJobPayload;

    use super::*;

    enum ProviderMode {
        Ok(Vec<SubtitleCandidate>),
        NotFound,
        Backend,
    }

    struct MockProvider {
        mode: ProviderMode,
    }

    impl SubtitleProvider for MockProvider {
        async fn search(
            &self,
            _query: &SubtitleQuery,
        ) -> Result<Vec<SubtitleCandidate>, SubtitleError> {
            match &self.mode {
                ProviderMode::Ok(candidates) => Ok(candidates.clone()),
                ProviderMode::NotFound => Err(SubtitleError::NotFound),
                ProviderMode::Backend => Err(SubtitleError::Backend("boom".into())),
            }
        }

        async fn download(&self, _file_id: &str) -> Result<FetchedSubtitle, SubtitleError> {
            Ok(FetchedSubtitle { content: "WEBVTT\n".into(), format: SubtitleFormat::Vtt })
        }
    }

    #[derive(Clone, Default)]
    struct MockStore {
        fail: bool,
        stored: Arc<Mutex<Vec<String>>>,
    }

    impl SubtitleStore for MockStore {
        async fn store(
            &self,
            version: &VersionId,
            file_id: &str,
            format: SubtitleFormat,
            _content: &str,
        ) -> Result<String, SubtitleError> {
            if self.fail {
                return Err(SubtitleError::Store("disk full".into()));
            }
            let path = format!("/subs/{}/{file_id}.{}", version.0, format.extension());
            self.stored.lock().unwrap().push(path.clone());
            Ok(path)
        }
    }

    #[derive(Clone, Default)]
    struct MockTrigger {
        fail: bool,
        calls: Arc<Mutex<usize>>,
    }

    impl TranslationTrigger for MockTrigger {
        async fn trigger(
            &self,
            _version_id: &VersionId,
            _parent: Option<&JobId>,
        ) -> Result<(), RepositoryError> {
            *self.calls.lock().unwrap() += 1;
            if self.fail { Err(RepositoryError::Backend("trigger boom".into())) } else { Ok(()) }
        }
    }

    #[derive(Clone, Default)]
    struct MockTranscription {
        fail: bool,
        calls: Arc<Mutex<usize>>,
    }

    impl TranscriptionTrigger for MockTranscription {
        async fn trigger(
            &self,
            _version_id: &VersionId,
            _parent: Option<&JobId>,
        ) -> Result<(), RepositoryError> {
            *self.calls.lock().unwrap() += 1;
            if self.fail { Err(RepositoryError::Backend("transcribe boom".into())) } else { Ok(()) }
        }
    }

    fn candidate(file_id: &str, language: &str) -> SubtitleCandidate {
        SubtitleCandidate {
            file_id: file_id.into(),
            language: Some(LanguageCode(language.into())),
            format: SubtitleFormat::Srt,
            release_name: None,
            download_count: None,
            rating: None,
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
            kind: JobKind::Subtitles,
            status: JobStatus::Running,
            priority: JobPriority::Normal,
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
        SubtitleJobPayload {
            version_id: VersionId("v1".into()),
            imdb_id: Some("tt1".into()),
            title: Some("The Matrix".into()),
            languages: vec!["en".into()],
            season: None,
            episode: None,
            transcribe_on_miss: false,
        }
        .encode()
    }

    fn payload_with_fallback() -> String {
        SubtitleJobPayload {
            version_id: VersionId("v1".into()),
            imdb_id: Some("tt1".into()),
            title: Some("The Matrix".into()),
            languages: vec!["en".into()],
            season: None,
            episode: None,
            transcribe_on_miss: true,
        }
        .encode()
    }

    #[tokio::test]
    async fn fetches_stores_and_merges_keeping_external_rows() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version());
        catalog
            .set_subtitle_files(
                &VersionId("v1".into()),
                &[SubtitleFile {
                    id: SubtitleFileId("sidecar".into()),
                    version: VersionId("v1".into()),
                    language: Some(LanguageCode("en".into())),
                    format: SubtitleFormat::Srt,
                    source: SubtitleSource::External,
                    path: "/m/v1.en.srt".into(),
                    translated_from: None,
                    label: None,
                    pinned: false,
                }],
            )
            .await
            .unwrap();
        let store = MockStore::default();
        let trigger = MockTrigger::default();
        let handler = SubtitlesJobHandler::new(
            MockProvider { mode: ProviderMode::Ok(vec![candidate("42", "en")]) },
            catalog.clone(),
            store.clone(),
            trigger.clone(),
            MockTranscription::default(),
        );

        handler.handle(&job(payload())).await.unwrap();

        assert_eq!(*trigger.calls.lock().unwrap(), 1);
        assert_eq!(store.stored.lock().unwrap().len(), 1);
        let detail = catalog.version_detail(&VersionId("v1".into())).await.unwrap().unwrap();
        assert_eq!(detail.subtitle_files.len(), 2);
        assert!(detail.subtitle_files.iter().any(|f| f.source == SubtitleSource::External));
        assert!(detail.subtitle_files.iter().any(|f| f.source == SubtitleSource::OpenSubtitles));
    }

    #[tokio::test]
    async fn missing_language_candidate_is_a_no_op() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version());
        let handler = SubtitlesJobHandler::new(
            MockProvider { mode: ProviderMode::Ok(vec![candidate("42", "fr")]) },
            catalog.clone(),
            MockStore::default(),
            MockTrigger::default(),
            MockTranscription::default(),
        );

        handler.handle(&job(payload())).await.unwrap();

        let detail = catalog.version_detail(&VersionId("v1".into())).await.unwrap().unwrap();
        assert!(detail.subtitle_files.is_empty());
    }

    #[tokio::test]
    async fn not_found_is_a_no_op() {
        let handler = SubtitlesJobHandler::new(
            MockProvider { mode: ProviderMode::NotFound },
            MockCatalogRepo::new(),
            MockStore::default(),
            MockTrigger::default(),
            MockTranscription::default(),
        );
        handler.handle(&job(payload())).await.unwrap();
    }

    #[tokio::test]
    async fn backend_search_failure_is_retryable() {
        let handler = SubtitlesJobHandler::new(
            MockProvider { mode: ProviderMode::Backend },
            MockCatalogRepo::new(),
            MockStore::default(),
            MockTrigger::default(),
            MockTranscription::default(),
        );
        assert!(matches!(
            handler.handle(&job(payload())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn store_failure_is_retryable() {
        let handler = SubtitlesJobHandler::new(
            MockProvider { mode: ProviderMode::Ok(vec![candidate("42", "en")]) },
            MockCatalogRepo::new(),
            MockStore { fail: true, ..MockStore::default() },
            MockTrigger::default(),
            MockTranscription::default(),
        );
        assert!(matches!(
            handler.handle(&job(payload())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn catalog_failure_is_retryable() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version());
        catalog.set_fail();
        let handler = SubtitlesJobHandler::new(
            MockProvider { mode: ProviderMode::Ok(vec![candidate("42", "en")]) },
            catalog,
            MockStore::default(),
            MockTrigger::default(),
            MockTranscription::default(),
        );
        assert!(matches!(
            handler.handle(&job(payload())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn invalid_payload_is_permanent() {
        let handler = SubtitlesJobHandler::new(
            MockProvider { mode: ProviderMode::NotFound },
            MockCatalogRepo::new(),
            MockStore::default(),
            MockTrigger::default(),
            MockTranscription::default(),
        );
        assert!(matches!(
            handler.handle(&job("garbage".into())).await.unwrap_err(),
            JobError::Permanent(_)
        ));
    }

    #[tokio::test]
    async fn translation_trigger_failure_is_retryable() {
        let handler = SubtitlesJobHandler::new(
            MockProvider { mode: ProviderMode::Ok(vec![candidate("42", "en")]) },
            MockCatalogRepo::new(),
            MockStore::default(),
            MockTrigger { fail: true, ..MockTrigger::default() },
            MockTranscription::default(),
        );
        assert!(matches!(
            handler.handle(&job(payload())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn not_found_triggers_transcription_when_fallback_enabled() {
        let transcription = MockTranscription::default();
        let handler = SubtitlesJobHandler::new(
            MockProvider { mode: ProviderMode::NotFound },
            MockCatalogRepo::new(),
            MockStore::default(),
            MockTrigger::default(),
            transcription.clone(),
        );

        handler.handle(&job(payload_with_fallback())).await.unwrap();

        assert_eq!(*transcription.calls.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn no_language_match_triggers_transcription_when_fallback_enabled() {
        let transcription = MockTranscription::default();
        let handler = SubtitlesJobHandler::new(
            MockProvider { mode: ProviderMode::Ok(vec![candidate("42", "fr")]) },
            MockCatalogRepo::new(),
            MockStore::default(),
            MockTrigger::default(),
            transcription.clone(),
        );

        handler.handle(&job(payload_with_fallback())).await.unwrap();

        assert_eq!(*transcription.calls.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn miss_does_not_transcribe_without_fallback() {
        let transcription = MockTranscription::default();
        let handler = SubtitlesJobHandler::new(
            MockProvider { mode: ProviderMode::NotFound },
            MockCatalogRepo::new(),
            MockStore::default(),
            MockTrigger::default(),
            transcription.clone(),
        );

        handler.handle(&job(payload())).await.unwrap();

        assert_eq!(*transcription.calls.lock().unwrap(), 0);
    }

    #[tokio::test]
    async fn successful_fetch_does_not_transcribe() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version());
        let transcription = MockTranscription::default();
        let handler = SubtitlesJobHandler::new(
            MockProvider { mode: ProviderMode::Ok(vec![candidate("42", "en")]) },
            catalog,
            MockStore::default(),
            MockTrigger::default(),
            transcription.clone(),
        );

        handler.handle(&job(payload_with_fallback())).await.unwrap();

        assert_eq!(*transcription.calls.lock().unwrap(), 0);
    }

    #[tokio::test]
    async fn transcription_trigger_failure_is_retryable() {
        let handler = SubtitlesJobHandler::new(
            MockProvider { mode: ProviderMode::NotFound },
            MockCatalogRepo::new(),
            MockStore::default(),
            MockTrigger::default(),
            MockTranscription { fail: true, ..MockTranscription::default() },
        );

        assert!(matches!(
            handler.handle(&job(payload_with_fallback())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn re_pull_prunes_translation_orphaned_by_replaced_source() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version());
        catalog
            .set_subtitle_files(
                &VersionId("v1".into()),
                &[
                    SubtitleFile {
                        id: SubtitleFileId("opensubtitles:v1:old".into()),
                        version: VersionId("v1".into()),
                        language: Some(LanguageCode("en".into())),
                        format: SubtitleFormat::Srt,
                        source: SubtitleSource::OpenSubtitles,
                        path: "/subs/old.srt".into(),
                        translated_from: None,
                        label: None,
                        pinned: false,
                    },
                    SubtitleFile {
                        id: SubtitleFileId("machine:v1:fr".into()),
                        version: VersionId("v1".into()),
                        language: Some(LanguageCode("fr".into())),
                        format: SubtitleFormat::Vtt,
                        source: SubtitleSource::MachineTranslated,
                        path: "/subs/fr.vtt".into(),
                        translated_from: Some(SubtitleFileId("opensubtitles:v1:old".into())),
                        label: None,
                        pinned: false,
                    },
                ],
            )
            .await
            .unwrap();
        let handler = SubtitlesJobHandler::new(
            MockProvider { mode: ProviderMode::Ok(vec![candidate("42", "en")]) },
            catalog.clone(),
            MockStore::default(),
            MockTrigger::default(),
            MockTranscription::default(),
        );

        handler.handle(&job(payload())).await.unwrap();

        let files =
            catalog.version_detail(&VersionId("v1".into())).await.unwrap().unwrap().subtitle_files;
        assert!(files.iter().all(|f| f.source != SubtitleSource::MachineTranslated));
        assert!(files.iter().any(|f| f.id.0 == "opensubtitles:v1:42"));
    }

    async fn re_pull_over(seeded: &[SubtitleFile]) -> Vec<SubtitleFile> {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version());
        catalog.set_subtitle_files(&VersionId("v1".into()), seeded).await.unwrap();
        let handler = SubtitlesJobHandler::new(
            MockProvider {
                mode: ProviderMode::Ok(vec![SubtitleCandidate {
                    release_name: Some("The.Matrix.WEB".into()),
                    ..candidate("42", "en")
                }]),
            },
            catalog.clone(),
            MockStore::default(),
            MockTrigger::default(),
            MockTranscription::default(),
        );
        handler.handle(&job(payload())).await.unwrap();
        catalog.version_detail(&VersionId("v1".into())).await.unwrap().unwrap().subtitle_files
    }

    fn downloaded(id: &str, pinned: bool, label: Option<&str>) -> SubtitleFile {
        SubtitleFile {
            id: SubtitleFileId(id.into()),
            version: VersionId("v1".into()),
            language: Some(LanguageCode("en".into())),
            format: SubtitleFormat::Srt,
            source: SubtitleSource::OpenSubtitles,
            path: format!("/subs/{id}.srt"),
            translated_from: None,
            label: label.map(str::to_owned),
            pinned,
        }
    }

    #[tokio::test]
    async fn a_pinned_file_survives_a_re_pull_that_replaces_the_unpinned_one() {
        let files = re_pull_over(&[
            downloaded("opensubtitles:v1:hand", true, Some("The.Matrix.BluRay")),
            downloaded("opensubtitles:v1:old", false, None),
        ])
        .await;

        assert_eq!(files.len(), 2);
        let pinned = files
            .iter()
            .find(|f| f.id.0 == "opensubtitles:v1:hand")
            .expect("the admin's pick must survive");
        assert!(pinned.pinned);
        assert_eq!(pinned.label.as_deref(), Some("The.Matrix.BluRay"));
        assert!(
            files.iter().all(|f| f.id.0 != "opensubtitles:v1:old"),
            "the job's own previous pick is still replaceable"
        );
        let fresh = files.iter().find(|f| f.id.0 == "opensubtitles:v1:42").unwrap();
        assert_eq!(fresh.label.as_deref(), Some("The.Matrix.WEB"));
        assert!(!fresh.pinned);
    }

    #[tokio::test]
    async fn a_pinned_file_the_job_picks_again_is_not_written_twice() {
        let files = re_pull_over(&[downloaded("opensubtitles:v1:42", true, Some("Kept"))]).await;

        assert_eq!(files.len(), 1);
        assert!(files[0].pinned, "the admin's row wins over the job's");
        assert_eq!(files[0].label.as_deref(), Some("Kept"));
    }
}

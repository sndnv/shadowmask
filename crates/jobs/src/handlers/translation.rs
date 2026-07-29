use domain::common::LanguageCode;
use domain::error::TranslationError;
use domain::job::Job;
use domain::media::{
    SubtitleFile, SubtitleFileId, SubtitleReader, SubtitleSource, SubtitleStore,
    TranslationProvider, TranslationRequest,
};
use domain::repository::CatalogRepository;
use services::library::TranslationJobPayload;

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct TranslationJobHandler<P, C, S> {
    provider: P,
    catalog: C,
    store: S,
}

impl<P, C, S> TranslationJobHandler<P, C, S> {
    pub fn new(provider: P, catalog: C, store: S) -> Self {
        Self {
            provider,
            catalog,
            store,
        }
    }
}

impl<P, C, S> JobHandler for TranslationJobHandler<P, C, S>
where
    P: TranslationProvider + Send + Sync,
    C: CatalogRepository + Send + Sync,
    S: SubtitleStore + SubtitleReader + Send + Sync,
{
    async fn handle(&self, job: &Job) -> Result<(), JobError> {
        let payload = TranslationJobPayload::decode(&job.payload)
            .map_err(|e| JobError::Permanent(format!("invalid translation payload: {e}")))?;

        let Some(detail) = self
            .catalog
            .version_detail(&payload.version_id)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?
        else {
            tracing::debug!("version gone; skipping translation");
            return Ok(());
        };
        let existing = detail.subtitle_files;

        let mut produced: Vec<SubtitleFile> = Vec::new();
        for target in &payload.target_languages {
            let target_language = LanguageCode(target.clone());
            if has_language(&existing, &target_language) {
                tracing::debug!("target already satisfied; skipping");
                continue;
            }
            let source = match &payload.source_subtitle_id {
                Some(id) => {
                    let Some(source) = existing.iter().find(|file| &file.id.0 == id) else {
                        tracing::debug!("chosen translation source not found; skipping target");
                        continue;
                    };
                    source
                }
                None => {
                    let Some(source) = pick_source(&existing, &target_language) else {
                        tracing::debug!("no translation source available; skipping target");
                        continue;
                    };
                    source
                }
            };
            let content = self
                .store
                .load(&source.path)
                .await
                .map_err(|e| JobError::Retryable(e.to_string()))?;
            let request = TranslationRequest {
                content,
                format: source.format,
                source_language: source.language.clone(),
                target_language: target_language.clone(),
            };
            let source_language = source
                .language
                .as_ref()
                .map(|code| code.0.as_str())
                .unwrap_or("unknown");
            tracing::info!(
                "translating version [{}] subtitle [{}] ({}) to [{}]",
                payload.version_id.0,
                source.id.0,
                source_language,
                target_language.0
            );
            let translated = match self.provider.translate(&request).await {
                Ok(translated) => {
                    tracing::info!(
                        "translation to [{}] produced a [{:?}] subtitle for version [{}]",
                        target_language.0,
                        translated.format,
                        payload.version_id.0
                    );
                    translated
                }
                Err(TranslationError::Unsupported(reason)) => {
                    tracing::debug!("translation unsupported: {reason}");
                    continue;
                }
                Err(e) => return Err(JobError::Retryable(e.to_string())),
            };
            if !translated.has_text() {
                tracing::debug!(
                    "translation to [{}] produced no usable text; skipping",
                    target_language.0
                );
                continue;
            }
            let path = self
                .store
                .store(
                    &payload.version_id,
                    &format!("machine:{}", target_language.0),
                    translated.format,
                    &translated.content,
                )
                .await
                .map_err(|e| JobError::Retryable(e.to_string()))?;
            produced.push(SubtitleFile {
                id: SubtitleFileId(format!(
                    "machine:{}:{}",
                    payload.version_id.0, target_language.0
                )),
                version: payload.version_id.clone(),
                language: Some(target_language),
                format: translated.format,
                source: SubtitleSource::MachineTranslated,
                path,
                translated_from: Some(source.id.clone()),
            });
        }

        if produced.is_empty() {
            return Ok(());
        }
        let mut merged = existing;
        merged.extend(produced);
        self.catalog
            .set_subtitle_files(&payload.version_id, &merged)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))
    }
}

fn has_language(files: &[SubtitleFile], language: &LanguageCode) -> bool {
    files.iter().any(|file| {
        matches!(
            file.source,
            SubtitleSource::External | SubtitleSource::MachineTranslated
        ) && file.language.as_ref() == Some(language)
    })
}

fn pick_source<'a>(files: &'a [SubtitleFile], target: &LanguageCode) -> Option<&'a SubtitleFile> {
    const ORDER: [SubtitleSource; 3] = [
        SubtitleSource::External,
        SubtitleSource::OpenSubtitles,
        SubtitleSource::Generated,
    ];
    ORDER.into_iter().find_map(|source| {
        files
            .iter()
            .find(|file| file.source == source && file.language.as_ref() != Some(target))
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use domain::catalog::{TitleId, Version, VersionId};
    use domain::common::Quality;
    use domain::error::SubtitleError;
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use domain::media::{FetchedSubtitle, SubtitleFormat};
    use jiff::Timestamp;
    use services::library::TranslationJobPayload;
    use services::mock::MockCatalogRepo;

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

    impl TranslationProvider for MockProvider {
        async fn translate(
            &self,
            request: &TranslationRequest,
        ) -> Result<FetchedSubtitle, TranslationError> {
            match &self.mode {
                ProviderMode::Ok => Ok(FetchedSubtitle {
                    content: format!(
                        "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\n{}\n",
                        request.target_language.0
                    ),
                    format: SubtitleFormat::Vtt,
                }),
                ProviderMode::Empty => Ok(FetchedSubtitle {
                    content: "WEBVTT\n".into(),
                    format: SubtitleFormat::Vtt,
                }),
                ProviderMode::Unsupported => Err(TranslationError::Unsupported("no engine".into())),
                ProviderMode::Backend => Err(TranslationError::Backend("boom".into())),
            }
        }
    }

    #[derive(Default)]
    struct MockStore {
        store_fail: bool,
        load_fail: bool,
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
        ) -> Result<String, SubtitleError> {
            if self.store_fail {
                return Err(SubtitleError::Store("disk full".into()));
            }
            let path = format!("/subs/{}/{file_id}.{}", version.0, format.extension());
            self.stored.lock().unwrap().push(path.clone());
            if let Some(catalog) = &self.fail_catalog_after {
                catalog.set_fail();
            }
            Ok(path)
        }
    }

    impl SubtitleReader for MockStore {
        async fn load(&self, _path: &str) -> Result<String, SubtitleError> {
            if self.load_fail {
                return Err(SubtitleError::Backend("read failed".into()));
            }
            Ok("WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nhello\n".to_owned())
        }
    }

    fn sub(id: &str, source: SubtitleSource, language: Option<&str>) -> SubtitleFile {
        SubtitleFile {
            id: SubtitleFileId(id.into()),
            version: VersionId("v1".into()),
            language: language.map(|code| LanguageCode(code.into())),
            format: SubtitleFormat::Vtt,
            source,
            path: format!("/subs/{id}.vtt"),
            translated_from: None,
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
            edition: None,
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn job(raw: String) -> Job {
        let now = Timestamp::UNIX_EPOCH;
        Job {
            id: JobId("job-1".into()),
            kind: JobKind::Translation,
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

    fn payload(targets: &[&str]) -> String {
        TranslationJobPayload {
            version_id: VersionId("v1".into()),
            target_languages: targets.iter().map(|t| (*t).to_owned()).collect(),
            source_subtitle_id: None,
        }
        .encode()
        .unwrap()
    }

    fn payload_with_source(source_id: &str, target: &str) -> String {
        TranslationJobPayload {
            version_id: VersionId("v1".into()),
            target_languages: vec![target.to_owned()],
            source_subtitle_id: Some(source_id.to_owned()),
        }
        .encode()
        .unwrap()
    }

    async fn seed(catalog: &MockCatalogRepo, files: &[SubtitleFile]) {
        catalog.add_version(version());
        if !files.is_empty() {
            catalog
                .set_subtitle_files(&VersionId("v1".into()), files)
                .await
                .unwrap();
        }
    }

    async fn files_of(catalog: &MockCatalogRepo) -> Vec<SubtitleFile> {
        catalog
            .version_detail(&VersionId("v1".into()))
            .await
            .unwrap()
            .unwrap()
            .subtitle_files
    }

    fn handler(
        mode: ProviderMode,
        catalog: MockCatalogRepo,
        store: MockStore,
    ) -> TranslationJobHandler<MockProvider, MockCatalogRepo, MockStore> {
        TranslationJobHandler::new(MockProvider { mode }, catalog, store)
    }

    #[tokio::test]
    async fn produces_machine_translation_alongside_opensubtitles() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[
                sub("os-en", SubtitleSource::OpenSubtitles, Some("en")),
                sub("os-fr", SubtitleSource::OpenSubtitles, Some("fr")),
            ],
        )
        .await;
        let handler = handler(ProviderMode::Ok, catalog.clone(), MockStore::default());

        handler.handle(&job(payload(&["fr"]))).await.unwrap();

        let files = files_of(&catalog).await;
        assert_eq!(files.len(), 3);
        assert!(
            files
                .iter()
                .any(|f| f.source == SubtitleSource::MachineTranslated
                    && f.language == Some(LanguageCode("fr".into())))
        );
        assert!(
            files
                .iter()
                .any(|f| f.source == SubtitleSource::OpenSubtitles
                    && f.language == Some(LanguageCode("fr".into())))
        );
    }

    #[tokio::test]
    async fn records_translation_source_provenance() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[sub("os-en", SubtitleSource::OpenSubtitles, Some("en"))],
        )
        .await;
        let handler = handler(ProviderMode::Ok, catalog.clone(), MockStore::default());

        handler.handle(&job(payload(&["fr"]))).await.unwrap();

        let mt = files_of(&catalog)
            .await
            .into_iter()
            .find(|f| f.source == SubtitleSource::MachineTranslated)
            .unwrap();
        assert_eq!(mt.translated_from, Some(SubtitleFileId("os-en".into())));
    }

    #[tokio::test]
    async fn empty_translation_is_not_stored() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[sub("os-en", SubtitleSource::OpenSubtitles, Some("en"))],
        )
        .await;
        let store = MockStore::default();
        let handler = handler(
            ProviderMode::Empty,
            catalog.clone(),
            MockStore {
                stored: store.stored.clone(),
                ..MockStore::default()
            },
        );

        handler.handle(&job(payload(&["fr"]))).await.unwrap();

        assert!(store.stored.lock().unwrap().is_empty());
        assert!(
            files_of(&catalog)
                .await
                .iter()
                .all(|f| f.source != SubtitleSource::MachineTranslated)
        );
    }

    #[tokio::test]
    async fn explicit_source_translates_even_when_auto_pick_would_skip() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[sub("gen-de", SubtitleSource::Generated, Some("de"))],
        )
        .await;
        let handler = handler(ProviderMode::Ok, catalog.clone(), MockStore::default());

        handler
            .handle(&job(payload_with_source("gen-de", "de")))
            .await
            .unwrap();

        let files = files_of(&catalog).await;
        assert!(
            files
                .iter()
                .any(|f| f.source == SubtitleSource::MachineTranslated)
        );
    }

    #[tokio::test]
    async fn explicit_source_missing_is_a_noop() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[sub("gen-de", SubtitleSource::Generated, Some("de"))],
        )
        .await;
        let store = MockStore::default();
        let handler = handler(
            ProviderMode::Ok,
            catalog.clone(),
            MockStore {
                stored: store.stored.clone(),
                ..MockStore::default()
            },
        );

        handler
            .handle(&job(payload_with_source("absent", "de")))
            .await
            .unwrap();

        assert!(store.stored.lock().unwrap().is_empty());
        assert_eq!(files_of(&catalog).await.len(), 1);
    }

    #[tokio::test]
    async fn translates_from_generated_source_with_unknown_language() {
        let catalog = MockCatalogRepo::new();
        seed(&catalog, &[sub("gen", SubtitleSource::Generated, None)]).await;
        let handler = handler(ProviderMode::Ok, catalog.clone(), MockStore::default());

        handler.handle(&job(payload(&["fr"]))).await.unwrap();

        let files = files_of(&catalog).await;
        assert!(
            files
                .iter()
                .any(|f| f.source == SubtitleSource::MachineTranslated)
        );
    }

    #[tokio::test]
    async fn skips_target_with_native_subtitle() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[sub("ext", SubtitleSource::External, Some("fr"))],
        )
        .await;
        let store = MockStore::default();
        let handler = handler(
            ProviderMode::Ok,
            catalog.clone(),
            MockStore {
                stored: store.stored.clone(),
                ..MockStore::default()
            },
        );

        handler.handle(&job(payload(&["fr"]))).await.unwrap();

        assert!(store.stored.lock().unwrap().is_empty());
        assert_eq!(files_of(&catalog).await.len(), 1);
    }

    #[tokio::test]
    async fn skips_target_already_machine_translated() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[
                sub("os-en", SubtitleSource::OpenSubtitles, Some("en")),
                sub("mt-fr", SubtitleSource::MachineTranslated, Some("fr")),
            ],
        )
        .await;
        let store = MockStore::default();
        let handler = handler(
            ProviderMode::Ok,
            catalog.clone(),
            MockStore {
                stored: store.stored.clone(),
                ..MockStore::default()
            },
        );

        handler.handle(&job(payload(&["fr"]))).await.unwrap();

        assert!(store.stored.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn skips_target_without_any_source() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[sub("os-fr", SubtitleSource::OpenSubtitles, Some("fr"))],
        )
        .await;
        let store = MockStore::default();
        let handler = handler(
            ProviderMode::Ok,
            catalog.clone(),
            MockStore {
                stored: store.stored.clone(),
                ..MockStore::default()
            },
        );

        handler.handle(&job(payload(&["fr"]))).await.unwrap();

        assert!(store.stored.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn unsupported_skips_target() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[sub("os-en", SubtitleSource::OpenSubtitles, Some("en"))],
        )
        .await;
        let handler = handler(
            ProviderMode::Unsupported,
            catalog.clone(),
            MockStore::default(),
        );

        handler.handle(&job(payload(&["fr"]))).await.unwrap();

        assert!(
            files_of(&catalog)
                .await
                .iter()
                .all(|f| f.source != SubtitleSource::MachineTranslated)
        );
    }

    #[tokio::test]
    async fn backend_failure_is_retryable() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[sub("os-en", SubtitleSource::OpenSubtitles, Some("en"))],
        )
        .await;
        let handler = handler(ProviderMode::Backend, catalog, MockStore::default());
        assert!(matches!(
            handler.handle(&job(payload(&["fr"]))).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn load_failure_is_retryable() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[sub("os-en", SubtitleSource::OpenSubtitles, Some("en"))],
        )
        .await;
        let handler = handler(
            ProviderMode::Ok,
            catalog,
            MockStore {
                load_fail: true,
                ..MockStore::default()
            },
        );
        assert!(matches!(
            handler.handle(&job(payload(&["fr"]))).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn store_failure_is_retryable() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[sub("os-en", SubtitleSource::OpenSubtitles, Some("en"))],
        )
        .await;
        let handler = handler(
            ProviderMode::Ok,
            catalog,
            MockStore {
                store_fail: true,
                ..MockStore::default()
            },
        );
        assert!(matches!(
            handler.handle(&job(payload(&["fr"]))).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn catalog_read_failure_is_retryable() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[sub("os-en", SubtitleSource::OpenSubtitles, Some("en"))],
        )
        .await;
        catalog.set_fail();
        let handler = handler(ProviderMode::Ok, catalog, MockStore::default());
        assert!(matches!(
            handler.handle(&job(payload(&["fr"]))).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn catalog_write_failure_is_retryable() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[sub("os-en", SubtitleSource::OpenSubtitles, Some("en"))],
        )
        .await;
        let handler = handler(
            ProviderMode::Ok,
            catalog.clone(),
            MockStore {
                fail_catalog_after: Some(catalog),
                ..MockStore::default()
            },
        );
        assert!(matches!(
            handler.handle(&job(payload(&["fr"]))).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn version_missing_is_a_no_op() {
        let handler = handler(
            ProviderMode::Ok,
            MockCatalogRepo::new(),
            MockStore::default(),
        );
        handler.handle(&job(payload(&["fr"]))).await.unwrap();
    }

    #[tokio::test]
    async fn invalid_payload_is_permanent() {
        let handler = handler(
            ProviderMode::Ok,
            MockCatalogRepo::new(),
            MockStore::default(),
        );
        assert!(matches!(
            handler.handle(&job("garbage".into())).await.unwrap_err(),
            JobError::Permanent(_)
        ));
    }

    #[tokio::test]
    async fn subtitle_store_default_remove_is_a_noop() {
        assert!(MockStore::default().remove("/anything").await.is_ok());
    }

    #[test]
    fn pick_source_prefers_external_then_opensubtitles_then_generated() {
        let fr = LanguageCode("fr".into());
        let external = sub("ext", SubtitleSource::External, Some("en"));
        let opensubs = sub("os", SubtitleSource::OpenSubtitles, Some("en"));
        let generated = sub("gen", SubtitleSource::Generated, None);

        let all = [external.clone(), opensubs.clone(), generated.clone()];
        assert_eq!(pick_source(&all, &fr).unwrap().id, external.id);

        let without_external = [opensubs.clone(), generated.clone()];
        assert_eq!(pick_source(&without_external, &fr).unwrap().id, opensubs.id);

        let only_generated = [generated.clone()];
        assert_eq!(pick_source(&only_generated, &fr).unwrap().id, generated.id);
    }

    #[test]
    fn pick_source_excludes_target_language_and_returns_none() {
        let fr = LanguageCode("fr".into());
        let only_target = [sub("os-fr", SubtitleSource::OpenSubtitles, Some("fr"))];
        assert!(pick_source(&only_target, &fr).is_none());
    }
}

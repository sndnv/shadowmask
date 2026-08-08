use domain::common::LanguageCode;
use domain::job::Job;
use domain::media::{
    FetchedSubtitle, SubtitleCombiner, SubtitleFile, SubtitleFileId, SubtitleReader,
    SubtitleSource, SubtitleStore,
};
use domain::repository::CatalogRepository;
use services::library::CombineJobPayload;

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct CombineJobHandler<Cmb, C, S> {
    combiner: Cmb,
    catalog: C,
    store: S,
}

impl<Cmb, C, S> CombineJobHandler<Cmb, C, S> {
    pub fn new(combiner: Cmb, catalog: C, store: S) -> Self {
        Self {
            combiner,
            catalog,
            store,
        }
    }
}

impl<Cmb, C, S> JobHandler for CombineJobHandler<Cmb, C, S>
where
    Cmb: SubtitleCombiner + Send + Sync,
    C: CatalogRepository + Send + Sync,
    S: SubtitleStore + SubtitleReader + Send + Sync,
{
    async fn handle(&self, job: &Job) -> Result<(), JobError> {
        let payload = CombineJobPayload::decode(&job.payload)
            .map_err(|e| JobError::Permanent(format!("invalid combine payload: {e}")))?;

        let Some(detail) = self
            .catalog
            .version_detail(&payload.version_id)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?
        else {
            tracing::debug!("version gone; skipping combine");
            return Ok(());
        };
        let existing = detail.subtitle_files;

        let Some(top) = existing
            .iter()
            .find(|file| file.id.0 == payload.top_subtitle_id)
        else {
            tracing::debug!("top subtitle not found; skipping combine");
            return Ok(());
        };
        let Some(bottom) = existing
            .iter()
            .find(|file| file.id.0 == payload.bottom_subtitle_id)
        else {
            tracing::debug!("bottom subtitle not found; skipping combine");
            return Ok(());
        };

        let combined_id = SubtitleFileId(format!(
            "combined:{}:{}:{}",
            payload.version_id.0, payload.top_subtitle_id, payload.bottom_subtitle_id
        ));
        if existing.iter().any(|file| file.id == combined_id) {
            tracing::debug!("combined subtitle already present; skipping");
            return Ok(());
        }

        tracing::info!(
            "combining subtitles [{}] + [{}] for version [{}]",
            payload.top_subtitle_id,
            payload.bottom_subtitle_id,
            payload.version_id.0
        );
        let language = Some(LanguageCode(format!(
            "{}+{}",
            language_code(top),
            language_code(bottom)
        )));
        let top_input = self.load(top).await?;
        let bottom_input = self.load(bottom).await?;

        let combined = self
            .combiner
            .combine(&top_input, &bottom_input)
            .map_err(|e| JobError::Retryable(e.to_string()))?;

        let path = self
            .store
            .store(
                &payload.version_id,
                &format!(
                    "combined:{}:{}",
                    payload.top_subtitle_id, payload.bottom_subtitle_id
                ),
                combined.format,
                &combined.content,
            )
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?;

        let produced = SubtitleFile {
            id: combined_id,
            version: payload.version_id.clone(),
            language,
            format: combined.format,
            source: SubtitleSource::Combined,
            path,
            translated_from: None,
        };

        let mut merged = existing;
        merged.push(produced);
        self.catalog
            .set_subtitle_files(&payload.version_id, &merged)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))
    }
}

impl<Cmb, C, S> CombineJobHandler<Cmb, C, S>
where
    S: SubtitleReader + Send + Sync,
{
    async fn load(&self, file: &SubtitleFile) -> Result<FetchedSubtitle, JobError> {
        let content = self
            .store
            .load(&file.path)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?;
        Ok(FetchedSubtitle {
            content,
            format: file.format,
        })
    }
}

fn language_code(file: &SubtitleFile) -> String {
    file.language
        .as_ref()
        .map(|code| code.0.clone())
        .unwrap_or_else(|| "und".to_owned())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use domain::catalog::{TitleId, Version, VersionId};
    use domain::common::Quality;
    use domain::error::SubtitleError;
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use domain::media::SubtitleFormat;
    use jiff::Timestamp;
    use services::library::CombineJobPayload;
    use services::mock::MockCatalogRepo;

    use super::*;

    struct MockCombiner {
        fail: bool,
    }

    impl SubtitleCombiner for MockCombiner {
        fn combine(
            &self,
            top: &FetchedSubtitle,
            bottom: &FetchedSubtitle,
        ) -> Result<FetchedSubtitle, SubtitleError> {
            if self.fail {
                return Err(SubtitleError::Parse("bad subtitle".into()));
            }
            Ok(FetchedSubtitle {
                content: format!("{}|{}", top.content, bottom.content),
                format: SubtitleFormat::Vtt,
            })
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
            kind: JobKind::Combine,
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

    fn payload(top: &str, bottom: &str) -> String {
        CombineJobPayload {
            version_id: VersionId("v1".into()),
            top_subtitle_id: top.to_owned(),
            bottom_subtitle_id: bottom.to_owned(),
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
        combiner: MockCombiner,
        catalog: MockCatalogRepo,
        store: MockStore,
    ) -> CombineJobHandler<MockCombiner, MockCatalogRepo, MockStore> {
        CombineJobHandler::new(combiner, catalog, store)
    }

    #[tokio::test]
    async fn combines_primary_and_secondary_into_a_combined_track() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[
                sub("sf-en", SubtitleSource::External, Some("en")),
                sub("sf-fr", SubtitleSource::External, Some("fr")),
            ],
        )
        .await;
        let handler = handler(
            MockCombiner { fail: false },
            catalog.clone(),
            MockStore::default(),
        );

        handler
            .handle(&job(payload("sf-en", "sf-fr")))
            .await
            .unwrap();

        let files = files_of(&catalog).await;
        assert_eq!(files.len(), 3);
        let combined = files
            .iter()
            .find(|f| f.source == SubtitleSource::Combined)
            .unwrap();
        assert_eq!(combined.language, Some(LanguageCode("en+fr".into())));
        assert_eq!(combined.format, SubtitleFormat::Vtt);
    }

    #[tokio::test]
    async fn missing_language_falls_back_to_und() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[
                sub("sf-en", SubtitleSource::External, Some("en")),
                sub("sf-x", SubtitleSource::Generated, None),
            ],
        )
        .await;
        let handler = handler(
            MockCombiner { fail: false },
            catalog.clone(),
            MockStore::default(),
        );

        handler
            .handle(&job(payload("sf-en", "sf-x")))
            .await
            .unwrap();

        let files = files_of(&catalog).await;
        let combined = files
            .iter()
            .find(|f| f.source == SubtitleSource::Combined)
            .unwrap();
        assert_eq!(combined.language, Some(LanguageCode("en+und".into())));
    }

    #[tokio::test]
    async fn missing_primary_is_a_noop() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[sub("sf-fr", SubtitleSource::External, Some("fr"))],
        )
        .await;
        let store = MockStore::default();
        let handler = handler(
            MockCombiner { fail: false },
            catalog.clone(),
            MockStore {
                stored: store.stored.clone(),
                ..MockStore::default()
            },
        );

        handler
            .handle(&job(payload("absent", "sf-fr")))
            .await
            .unwrap();

        assert!(store.stored.lock().unwrap().is_empty());
        assert_eq!(files_of(&catalog).await.len(), 1);
    }

    #[tokio::test]
    async fn missing_secondary_is_a_noop() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[sub("sf-en", SubtitleSource::External, Some("en"))],
        )
        .await;
        let store = MockStore::default();
        let handler = handler(
            MockCombiner { fail: false },
            catalog.clone(),
            MockStore {
                stored: store.stored.clone(),
                ..MockStore::default()
            },
        );

        handler
            .handle(&job(payload("sf-en", "absent")))
            .await
            .unwrap();

        assert!(store.stored.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn already_combined_is_a_noop() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[
                sub("sf-en", SubtitleSource::External, Some("en")),
                sub("sf-fr", SubtitleSource::External, Some("fr")),
                sub(
                    "combined:v1:sf-en:sf-fr",
                    SubtitleSource::Combined,
                    Some("en+fr"),
                ),
            ],
        )
        .await;
        let store = MockStore::default();
        let handler = handler(
            MockCombiner { fail: false },
            catalog.clone(),
            MockStore {
                stored: store.stored.clone(),
                ..MockStore::default()
            },
        );

        handler
            .handle(&job(payload("sf-en", "sf-fr")))
            .await
            .unwrap();

        assert!(store.stored.lock().unwrap().is_empty());
        assert_eq!(files_of(&catalog).await.len(), 3);
    }

    #[tokio::test]
    async fn combiner_failure_is_retryable() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[
                sub("sf-en", SubtitleSource::External, Some("en")),
                sub("sf-fr", SubtitleSource::External, Some("fr")),
            ],
        )
        .await;
        let handler = handler(MockCombiner { fail: true }, catalog, MockStore::default());
        assert!(matches!(
            handler
                .handle(&job(payload("sf-en", "sf-fr")))
                .await
                .unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn load_failure_is_retryable() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[
                sub("sf-en", SubtitleSource::External, Some("en")),
                sub("sf-fr", SubtitleSource::External, Some("fr")),
            ],
        )
        .await;
        let handler = handler(
            MockCombiner { fail: false },
            catalog,
            MockStore {
                load_fail: true,
                ..MockStore::default()
            },
        );
        assert!(matches!(
            handler
                .handle(&job(payload("sf-en", "sf-fr")))
                .await
                .unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn store_failure_is_retryable() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[
                sub("sf-en", SubtitleSource::External, Some("en")),
                sub("sf-fr", SubtitleSource::External, Some("fr")),
            ],
        )
        .await;
        let handler = handler(
            MockCombiner { fail: false },
            catalog,
            MockStore {
                store_fail: true,
                ..MockStore::default()
            },
        );
        assert!(matches!(
            handler
                .handle(&job(payload("sf-en", "sf-fr")))
                .await
                .unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn catalog_write_failure_is_retryable() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[
                sub("sf-en", SubtitleSource::External, Some("en")),
                sub("sf-fr", SubtitleSource::External, Some("fr")),
            ],
        )
        .await;
        let handler = handler(
            MockCombiner { fail: false },
            catalog.clone(),
            MockStore {
                fail_catalog_after: Some(catalog),
                ..MockStore::default()
            },
        );
        assert!(matches!(
            handler
                .handle(&job(payload("sf-en", "sf-fr")))
                .await
                .unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn catalog_read_failure_is_retryable() {
        let catalog = MockCatalogRepo::new();
        seed(
            &catalog,
            &[
                sub("sf-en", SubtitleSource::External, Some("en")),
                sub("sf-fr", SubtitleSource::External, Some("fr")),
            ],
        )
        .await;
        catalog.set_fail();
        let handler = handler(MockCombiner { fail: false }, catalog, MockStore::default());
        assert!(matches!(
            handler
                .handle(&job(payload("sf-en", "sf-fr")))
                .await
                .unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn version_missing_is_a_noop() {
        let handler = handler(
            MockCombiner { fail: false },
            MockCatalogRepo::new(),
            MockStore::default(),
        );
        handler
            .handle(&job(payload("sf-en", "sf-fr")))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn invalid_payload_is_permanent() {
        let handler = handler(
            MockCombiner { fail: false },
            MockCatalogRepo::new(),
            MockStore::default(),
        );
        assert!(matches!(
            handler.handle(&job("garbage".into())).await.unwrap_err(),
            JobError::Permanent(_)
        ));
    }
}

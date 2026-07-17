use domain::common::LanguageCode;
use domain::error::SubtitleError;
use domain::job::Job;
use domain::media::{
    SubtitleFile, SubtitleFileId, SubtitleProvider, SubtitleQuery, SubtitleSource, SubtitleStore,
};
use domain::repository::CatalogRepository;
use services::library::SubtitleJobPayload;

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct SubtitlesJobHandler<P, C, S> {
    provider: P,
    catalog: C,
    store: S,
}

impl<P, C, S> SubtitlesJobHandler<P, C, S> {
    pub fn new(provider: P, catalog: C, store: S) -> Self {
        Self {
            provider,
            catalog,
            store,
        }
    }
}

impl<P, C, S> JobHandler for SubtitlesJobHandler<P, C, S>
where
    P: SubtitleProvider + Send + Sync,
    C: CatalogRepository + Send + Sync,
    S: SubtitleStore + Send + Sync,
{
    async fn handle(&self, job: &Job) -> Result<(), JobError> {
        let payload = SubtitleJobPayload::decode(&job.payload)
            .map_err(|e| JobError::Permanent(format!("invalid subtitle payload: {e}")))?;
        let languages: Vec<LanguageCode> = payload
            .languages
            .iter()
            .map(|language| LanguageCode(language.clone()))
            .collect();
        let query = SubtitleQuery {
            imdb_id: payload.imdb_id.clone(),
            query: payload.title.clone(),
            languages: languages.clone(),
            season: payload.season,
            episode: payload.episode,
        };
        tracing::info!("fetching subtitles");
        let candidates = match self.provider.search(&query).await {
            Ok(candidates) => candidates,
            Err(SubtitleError::NotFound) => {
                tracing::debug!("no subtitles found");
                return Ok(());
            }
            Err(e) => return Err(JobError::Retryable(e.to_string())),
        };

        let mut fetched: Vec<SubtitleFile> = Vec::new();
        for language in &languages {
            let Some(candidate) = candidates
                .iter()
                .find(|candidate| candidate.language.as_ref() == Some(language))
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
                .store(
                    &payload.version_id,
                    &candidate.file_id,
                    subtitle.format,
                    &subtitle.content,
                )
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
            });
        }

        if fetched.is_empty() {
            return Ok(());
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
            .filter(|file| file.source != SubtitleSource::OpenSubtitles)
            .collect();
        merged.extend(fetched);
        self.catalog
            .set_subtitle_files(&payload.version_id, &merged)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use domain::catalog::{TitleId, Version, VersionId};
    use domain::common::Quality;
    use domain::error::SubtitleError;
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use domain::media::{
        FetchedSubtitle, SubtitleCandidate, SubtitleFile, SubtitleFileId, SubtitleFormat,
        SubtitleQuery, SubtitleSource,
    };
    use jiff::Timestamp;
    use services::library::SubtitleJobPayload;
    use services::mock::MockCatalogRepo;

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
            Ok(FetchedSubtitle {
                content: "WEBVTT\n".into(),
                format: SubtitleFormat::Vtt,
            })
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

    fn candidate(file_id: &str, language: &str) -> SubtitleCandidate {
        SubtitleCandidate {
            file_id: file_id.into(),
            language: Some(LanguageCode(language.into())),
            format: SubtitleFormat::Srt,
            release_name: None,
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
        }
        .encode()
        .unwrap()
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
                }],
            )
            .await
            .unwrap();
        let store = MockStore::default();
        let handler = SubtitlesJobHandler::new(
            MockProvider {
                mode: ProviderMode::Ok(vec![candidate("42", "en")]),
            },
            catalog.clone(),
            store.clone(),
        );

        handler.handle(&job(payload())).await.unwrap();

        assert_eq!(store.stored.lock().unwrap().len(), 1);
        let detail = catalog
            .version_detail(&VersionId("v1".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(detail.subtitle_files.len(), 2);
        assert!(
            detail
                .subtitle_files
                .iter()
                .any(|f| f.source == SubtitleSource::External)
        );
        assert!(
            detail
                .subtitle_files
                .iter()
                .any(|f| f.source == SubtitleSource::OpenSubtitles)
        );
    }

    #[tokio::test]
    async fn missing_language_candidate_is_a_no_op() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version());
        let handler = SubtitlesJobHandler::new(
            MockProvider {
                mode: ProviderMode::Ok(vec![candidate("42", "fr")]),
            },
            catalog.clone(),
            MockStore::default(),
        );

        handler.handle(&job(payload())).await.unwrap();

        let detail = catalog
            .version_detail(&VersionId("v1".into()))
            .await
            .unwrap()
            .unwrap();
        assert!(detail.subtitle_files.is_empty());
    }

    #[tokio::test]
    async fn not_found_is_a_no_op() {
        let handler = SubtitlesJobHandler::new(
            MockProvider {
                mode: ProviderMode::NotFound,
            },
            MockCatalogRepo::new(),
            MockStore::default(),
        );
        handler.handle(&job(payload())).await.unwrap();
    }

    #[tokio::test]
    async fn backend_search_failure_is_retryable() {
        let handler = SubtitlesJobHandler::new(
            MockProvider {
                mode: ProviderMode::Backend,
            },
            MockCatalogRepo::new(),
            MockStore::default(),
        );
        assert!(matches!(
            handler.handle(&job(payload())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn store_failure_is_retryable() {
        let handler = SubtitlesJobHandler::new(
            MockProvider {
                mode: ProviderMode::Ok(vec![candidate("42", "en")]),
            },
            MockCatalogRepo::new(),
            MockStore {
                fail: true,
                ..MockStore::default()
            },
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
            MockProvider {
                mode: ProviderMode::Ok(vec![candidate("42", "en")]),
            },
            catalog,
            MockStore::default(),
        );
        assert!(matches!(
            handler.handle(&job(payload())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn invalid_payload_is_permanent() {
        let handler = SubtitlesJobHandler::new(
            MockProvider {
                mode: ProviderMode::NotFound,
            },
            MockCatalogRepo::new(),
            MockStore::default(),
        );
        assert!(matches!(
            handler.handle(&job("garbage".into())).await.unwrap_err(),
            JobError::Permanent(_)
        ));
    }
}

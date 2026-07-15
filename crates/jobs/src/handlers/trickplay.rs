use domain::job::Job;
use domain::media::TrickplayGenerator;
use domain::repository::CatalogRepository;
use services::library::TrickplayJobPayload;

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct TrickplayJobHandler<C, G> {
    catalog: C,
    generator: G,
}

impl<C, G> TrickplayJobHandler<C, G> {
    pub fn new(catalog: C, generator: G) -> Self {
        Self { catalog, generator }
    }
}

impl<C, G> JobHandler for TrickplayJobHandler<C, G>
where
    C: CatalogRepository + Send + Sync,
    G: TrickplayGenerator + Send + Sync,
{
    async fn handle(&self, job: &Job) -> Result<(), JobError> {
        let payload = TrickplayJobPayload::decode(&job.payload)
            .map_err(|e| JobError::Permanent(format!("invalid trickplay payload: {e}")))?;
        let asset = self
            .generator
            .generate(
                &payload.source_path,
                &payload.version_id,
                payload.duration_ms,
            )
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?;
        self.catalog
            .set_trickplay(&payload.version_id, &[asset])
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use domain::catalog::{TitleId, Version, VersionId};
    use domain::common::Quality;
    use domain::error::TrickplayError;
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use domain::media::TrickplayAsset;
    use jiff::Timestamp;
    use services::library::TrickplayJobPayload;
    use services::mock::MockCatalogRepo;

    use super::*;

    enum GenerationMode {
        Ok,
        Backend,
    }

    struct MockGenerator {
        mode: GenerationMode,
    }

    impl TrickplayGenerator for MockGenerator {
        async fn generate(
            &self,
            _input_path: &str,
            version: &VersionId,
            _duration_ms: u64,
        ) -> Result<TrickplayAsset, TrickplayError> {
            match self.mode {
                GenerationMode::Ok => Ok(TrickplayAsset {
                    version: version.clone(),
                    interval_ms: 10_000,
                    columns: 10,
                    rows: 10,
                    tile_width: 320,
                    tile_height: 180,
                    sheet_paths: vec!["/tp/v1/sheet-001.jpg".into()],
                }),
                GenerationMode::Backend => Err(TrickplayError::Backend("ffmpeg".into())),
            }
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
        }
    }

    fn job(raw: String) -> Job {
        let now = Timestamp::UNIX_EPOCH;
        Job {
            id: JobId("job-1".into()),
            kind: JobKind::Trickplay,
            status: JobStatus::Running,
            priority: JobPriority::Normal,
            payload: raw,
            attempts: 0,
            progress: 0.0,
            available_at: now,
            last_error: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn payload() -> String {
        TrickplayJobPayload {
            version_id: VersionId("v1".into()),
            source_path: "/m/v1.mkv".into(),
            duration_ms: 1000,
        }
        .encode()
        .unwrap()
    }

    #[tokio::test]
    async fn generates_and_persists_trickplay() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version());
        let handler = TrickplayJobHandler::new(
            catalog.clone(),
            MockGenerator {
                mode: GenerationMode::Ok,
            },
        );

        handler.handle(&job(payload())).await.unwrap();

        let detail = catalog
            .version_detail(&VersionId("v1".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(detail.trickplay.len(), 1);
        assert_eq!(detail.trickplay[0].columns, 10);
        assert_eq!(detail.trickplay[0].sheet_paths.len(), 1);
    }

    #[tokio::test]
    async fn backend_failure_is_retryable() {
        let handler = TrickplayJobHandler::new(
            MockCatalogRepo::new(),
            MockGenerator {
                mode: GenerationMode::Backend,
            },
        );
        assert!(matches!(
            handler.handle(&job(payload())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn set_trickplay_failure_is_retryable() {
        let catalog = MockCatalogRepo::new();
        catalog.set_fail();
        let handler = TrickplayJobHandler::new(
            catalog,
            MockGenerator {
                mode: GenerationMode::Ok,
            },
        );
        assert!(matches!(
            handler.handle(&job(payload())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn invalid_payload_is_permanent() {
        let handler = TrickplayJobHandler::new(
            MockCatalogRepo::new(),
            MockGenerator {
                mode: GenerationMode::Ok,
            },
        );
        assert!(matches!(
            handler.handle(&job("garbage".into())).await.unwrap_err(),
            JobError::Permanent(_)
        ));
    }
}

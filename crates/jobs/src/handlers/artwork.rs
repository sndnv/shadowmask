use domain::catalog::{ArtworkRef, ArtworkWidth};
use domain::error::ArtworkError;
use domain::job::Job;
use domain::metadata::{ARTWORK_WIDTHS, ArtworkPipeline, ArtworkSpec, ArtworkStore};
use domain::repository::CatalogRepository;
use services::library::ArtworkJobPayload;

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct ArtworkJobHandler<C, P, S> {
    catalog: C,
    pipeline: P,
    store: S,
}

impl<C, P, S> ArtworkJobHandler<C, P, S> {
    pub fn new(catalog: C, pipeline: P, store: S) -> Self {
        Self { catalog, pipeline, store }
    }
}

impl<C, P, S> JobHandler for ArtworkJobHandler<C, P, S>
where
    C: CatalogRepository + Send + Sync,
    P: ArtworkPipeline + Send + Sync,
    S: ArtworkStore + Send + Sync,
{
    async fn handle(&self, job: &Job) -> Result<(), JobError> {
        let payload = ArtworkJobPayload::decode(&job.payload)
            .map_err(|e| JobError::Permanent(format!("invalid artwork payload: {e}")))?;
        let owner = &payload.owner;
        let items = payload.items.len();
        tracing::info!("generating artwork for {} [{}], {items} items", owner.kind(), owner.id());

        let specs = ARTWORK_WIDTHS
            .map(|width| ArtworkSpec { max_width: width, max_height: width.saturating_mul(3) });

        let mut refs = Vec::new();
        for item in &payload.items {
            let rendered = match self.pipeline.process_all(&item.url, &specs).await {
                Ok(rendered) => rendered,
                Err(ArtworkError::Download(m)) | Err(ArtworkError::Store(m)) => {
                    return Err(JobError::Retryable(m));
                }
                Err(ArtworkError::Decode(_)) => continue,
            };
            let mut widths = Vec::with_capacity(rendered.len());
            for (&width, art) in ARTWORK_WIDTHS.iter().zip(&rendered) {
                self.store
                    .store(&item.id, width, art)
                    .await
                    .map_err(|e| JobError::Retryable(e.to_string()))?;
                widths.push(ArtworkWidth::new(
                    width,
                    self.store.path_for(&item.id, width, art.format).to_string_lossy().into_owned(),
                ));
            }
            refs.push(ArtworkRef { id: item.id.clone(), kind: item.kind, widths });
        }

        self.catalog
            .set_artwork(&payload.owner, &refs)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use domain::catalog::{ArtworkId, ArtworkOwner, MovieId};
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use domain::metadata::{ArtworkFormat, ArtworkKind, ProcessedArtwork};
    use jiff::Timestamp;
    use mocks::MockCatalogRepo;
    use services::library::{ArtworkJobItem, ArtworkJobPayload};

    use super::*;

    #[derive(Clone, Copy)]
    enum PipelineMode {
        Ok,
        Download,
        Decode,
    }

    #[derive(Clone)]
    struct MockPipeline {
        mode: PipelineMode,
        fetches: Arc<AtomicUsize>,
    }

    impl MockPipeline {
        fn new(mode: PipelineMode) -> Self {
            Self { mode, fetches: Arc::new(AtomicUsize::new(0)) }
        }
    }

    impl ArtworkPipeline for MockPipeline {
        async fn process_all(
            &self,
            _url: &str,
            specs: &[ArtworkSpec],
        ) -> Result<Vec<ProcessedArtwork>, ArtworkError> {
            self.fetches.fetch_add(1, Ordering::SeqCst);
            match self.mode {
                PipelineMode::Ok => Ok(specs
                    .iter()
                    .map(|spec| ProcessedArtwork {
                        bytes: vec![1, 2, 3],
                        width: spec.max_width,
                        height: spec.max_width,
                        format: ArtworkFormat::Jpeg,
                    })
                    .collect()),
                PipelineMode::Download => Err(ArtworkError::Download("network".into())),
                PipelineMode::Decode => Err(ArtworkError::Decode("bad image".into())),
            }
        }
    }

    #[derive(Clone, Default)]
    struct MockStore {
        stored: Arc<Mutex<Vec<(String, u32, ArtworkFormat)>>>,
        fail: bool,
    }

    impl ArtworkStore for MockStore {
        async fn store(
            &self,
            id: &ArtworkId,
            width: u32,
            art: &ProcessedArtwork,
        ) -> Result<(), ArtworkError> {
            if self.fail {
                return Err(ArtworkError::Store("disk full".into()));
            }
            self.stored.lock().unwrap().push((id.0.clone(), width, art.format));
            Ok(())
        }

        fn path_for(&self, id: &ArtworkId, width: u32, format: ArtworkFormat) -> PathBuf {
            PathBuf::from(&id.0).join(format!("{width}.{}", format.extension()))
        }
    }

    fn payload() -> ArtworkJobPayload {
        ArtworkJobPayload {
            owner: ArtworkOwner::Movie(MovieId("m1".into())),
            items: vec![ArtworkJobItem {
                id: ArtworkId("art-1".into()),
                kind: ArtworkKind::Poster,
                url: "https://cdn/poster.jpg".into(),
            }],
        }
    }

    fn job(raw: String) -> Job {
        let now = Timestamp::UNIX_EPOCH;
        Job {
            id: JobId("job-1".into()),
            kind: JobKind::Artwork,
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

    #[tokio::test]
    async fn stores_every_width_and_records_artwork() {
        let catalog = MockCatalogRepo::new();
        let store = MockStore::default();
        let handler = ArtworkJobHandler::new(
            catalog.clone(),
            MockPipeline::new(PipelineMode::Ok),
            store.clone(),
        );

        handler.handle(&job(payload().encode())).await.unwrap();

        assert_eq!(store.stored.lock().unwrap().len(), ARTWORK_WIDTHS.len());
        assert_eq!(
            store.path_for(&ArtworkId("art-1".into()), 480, ArtworkFormat::Jpeg),
            PathBuf::from("art-1/480.jpg")
        );
        let refs = catalog.list_artwork(&ArtworkOwner::Movie(MovieId("m1".into()))).await.unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].kind, ArtworkKind::Poster);
        assert_eq!(refs[0].sizes(), ARTWORK_WIDTHS.to_vec());
        assert_eq!(
            refs[0].widths[0].path, "art-1/180.jpg",
            "the sweep protects the file the store actually wrote"
        );
    }

    #[tokio::test]
    async fn every_rung_comes_from_one_fetch_of_the_source() {
        let store = MockStore::default();
        let pipeline = MockPipeline::new(PipelineMode::Ok);
        let handler =
            ArtworkJobHandler::new(MockCatalogRepo::new(), pipeline.clone(), store.clone());

        handler.handle(&job(payload().encode())).await.unwrap();

        assert_eq!(
            pipeline.fetches.load(Ordering::SeqCst),
            1,
            "one source image should be downloaded and decoded once, not once per width"
        );
        assert_eq!(store.stored.lock().unwrap().len(), ARTWORK_WIDTHS.len());
    }

    #[tokio::test]
    async fn a_decode_failure_on_one_item_does_not_abandon_the_rest() {
        let catalog = MockCatalogRepo::new();
        let store = MockStore::default();
        let mut payload = payload();
        payload.items.push(ArtworkJobItem {
            id: ArtworkId("art-2".into()),
            kind: ArtworkKind::Logo,
            url: "https://cdn/logo.png".into(),
        });
        let handler = ArtworkJobHandler::new(
            catalog.clone(),
            MockPipeline::new(PipelineMode::Decode),
            store.clone(),
        );

        handler.handle(&job(payload.encode())).await.unwrap();

        assert!(store.stored.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn download_failure_is_retryable() {
        let handler = ArtworkJobHandler::new(
            MockCatalogRepo::new(),
            MockPipeline::new(PipelineMode::Download),
            MockStore::default(),
        );
        assert!(matches!(
            handler.handle(&job(payload().encode())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn decode_failure_skips_the_image() {
        let catalog = MockCatalogRepo::new();
        let store = MockStore::default();
        let handler = ArtworkJobHandler::new(
            catalog.clone(),
            MockPipeline::new(PipelineMode::Decode),
            store.clone(),
        );

        handler.handle(&job(payload().encode())).await.unwrap();

        assert!(store.stored.lock().unwrap().is_empty());
        assert!(
            catalog
                .list_artwork(&ArtworkOwner::Movie(MovieId("m1".into())))
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn store_failure_is_retryable() {
        let store = MockStore { fail: true, ..MockStore::default() };
        let handler = ArtworkJobHandler::new(
            MockCatalogRepo::new(),
            MockPipeline::new(PipelineMode::Ok),
            store,
        );
        assert!(matches!(
            handler.handle(&job(payload().encode())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn set_artwork_failure_is_retryable() {
        let catalog = MockCatalogRepo::new();
        catalog.set_fail();
        let handler = ArtworkJobHandler::new(
            catalog,
            MockPipeline::new(PipelineMode::Ok),
            MockStore::default(),
        );
        assert!(matches!(
            handler.handle(&job(payload().encode())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn invalid_payload_is_permanent() {
        let handler = ArtworkJobHandler::new(
            MockCatalogRepo::new(),
            MockPipeline::new(PipelineMode::Ok),
            MockStore::default(),
        );
        assert!(matches!(
            handler.handle(&job("garbage".into())).await.unwrap_err(),
            JobError::Permanent(_)
        ));
    }
}

use domain::catalog::{ArtworkOwner, ArtworkRef, ArtworkWidth};
use domain::error::ArtworkError;
use domain::job::Job;
use domain::media::StillExtractor;
use domain::metadata::{
    ARTWORK_WIDTHS, ArtworkPipeline, ArtworkSpec, ArtworkStore, ProcessedArtwork,
};
use domain::repository::CatalogRepository;
use services::library::{ArtworkJobItem, ArtworkJobPayload, ArtworkSource};

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct ArtworkJobHandler<C, P, S, X> {
    catalog: C,
    pipeline: P,
    store: S,
    stills: X,
}

impl<C, P, S, X> ArtworkJobHandler<C, P, S, X> {
    pub fn new(catalog: C, pipeline: P, store: S, stills: X) -> Self {
        Self { catalog, pipeline, store, stills }
    }
}

impl<C, P, S, X> ArtworkJobHandler<C, P, S, X>
where
    P: ArtworkPipeline + Send + Sync,
    X: StillExtractor + Send + Sync,
{
    async fn render(
        &self,
        owner: &ArtworkOwner,
        item: &ArtworkJobItem,
        specs: &[ArtworkSpec],
    ) -> Result<Option<Vec<ProcessedArtwork>>, JobError> {
        let mut sources = item.sources.iter().peekable();
        while let Some(source) = sources.next() {
            match self.render_source(source, specs).await {
                Ok(rendered) => return Ok(Some(rendered)),
                Err(err) if sources.peek().is_some() => {
                    #[rustfmt::skip]
                    tracing::warn!("artwork source [{source:?}] for {} [{}] failed, trying the next: [{err}]", owner.kind(), owner.id());
                }
                Err(ArtworkError::Download(m) | ArtworkError::Store(m)) => {
                    return Err(JobError::Retryable(m));
                }
                Err(ArtworkError::Decode(m)) => {
                    #[rustfmt::skip]
                    tracing::warn!("artwork source [{source:?}] for {} [{}] skipped: [{m}]", owner.kind(), owner.id());
                    return Ok(None);
                }
            }
        }
        Ok(None)
    }

    async fn render_source(
        &self,
        source: &ArtworkSource,
        specs: &[ArtworkSpec],
    ) -> Result<Vec<ProcessedArtwork>, ArtworkError> {
        match source {
            ArtworkSource::Provider(url) => self.pipeline.process_all(url, specs).await,
            ArtworkSource::Thumbnail(url) => self.pipeline.process_jpeg(url, specs).await,
            ArtworkSource::Frame { path, offset_ms } => {
                let frame = self
                    .stills
                    .extract(path, *offset_ms)
                    .await
                    .map_err(|e| ArtworkError::Decode(e.to_string()))?;
                self.pipeline.process_bytes(frame, specs).await
            }
        }
    }
}

impl<C, P, S, X> JobHandler for ArtworkJobHandler<C, P, S, X>
where
    C: CatalogRepository + Send + Sync,
    P: ArtworkPipeline + Send + Sync,
    S: ArtworkStore + Send + Sync,
    X: StillExtractor + Send + Sync,
{
    async fn handle(&self, job: &Job) -> Result<(), JobError> {
        let payload = ArtworkJobPayload::decode(&job.payload)
            .map_err(|e| JobError::Permanent(format!("invalid artwork payload: {e}")))?;
        let owner = &payload.owner;
        let fallback = only_fallbacks(&payload);
        if fallback
            && !self
                .catalog
                .list_artwork(owner)
                .await
                .map_err(|e| JobError::Retryable(e.to_string()))?
                .is_empty()
        {
            #[rustfmt::skip]
            tracing::info!("{} [{}] has artwork by now, its fallback is skipped", owner.kind(), owner.id());
            return Ok(());
        }
        let items = payload.items.len();
        tracing::info!("generating artwork for {} [{}], {items} items", owner.kind(), owner.id());

        let specs = ARTWORK_WIDTHS
            .map(|width| ArtworkSpec { max_width: width, max_height: width.saturating_mul(3) });

        let mut refs = Vec::new();
        for item in &payload.items {
            let Some(rendered) = self.render(owner, item, &specs).await? else { continue };
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

        if !fallback {
            return self
                .catalog
                .set_artwork(owner, &refs)
                .await
                .map_err(|e| JobError::Retryable(e.to_string()));
        }
        let stored = self
            .catalog
            .set_artwork_if_empty(owner, &refs)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?;
        if !stored {
            #[rustfmt::skip]
            tracing::info!("{} [{}] got artwork while its fallback rendered, the fallback is dropped", owner.kind(), owner.id());
        }
        Ok(())
    }
}

fn only_fallbacks(payload: &ArtworkJobPayload) -> bool {
    payload
        .items
        .iter()
        .flat_map(|item| &item.sources)
        .all(|source| !matches!(source, ArtworkSource::Provider(_)))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use domain::catalog::{ArtworkId, ArtworkOwner, EpisodeId, MovieId};
    use domain::error::StillError;
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
        jpeg: PipelineMode,
        bytes: PipelineMode,
        fetches: Arc<AtomicUsize>,
        calls: Arc<Mutex<Vec<String>>>,
    }

    impl MockPipeline {
        fn new(mode: PipelineMode) -> Self {
            Self {
                mode,
                jpeg: mode,
                bytes: mode,
                fetches: Arc::new(AtomicUsize::new(0)),
                calls: Arc::default(),
            }
        }

        fn with_jpeg(self, jpeg: PipelineMode) -> Self {
            Self { jpeg, ..self }
        }

        fn with_bytes(self, bytes: PipelineMode) -> Self {
            Self { bytes, ..self }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }

        fn answer(
            &self,
            call: String,
            mode: PipelineMode,
            specs: &[ArtworkSpec],
        ) -> Result<Vec<ProcessedArtwork>, ArtworkError> {
            self.calls.lock().unwrap().push(call);
            match mode {
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

    impl ArtworkPipeline for MockPipeline {
        async fn process_all(
            &self,
            url: &str,
            specs: &[ArtworkSpec],
        ) -> Result<Vec<ProcessedArtwork>, ArtworkError> {
            self.fetches.fetch_add(1, Ordering::SeqCst);
            self.answer(format!("url {url}"), self.mode, specs)
        }

        async fn process_jpeg(
            &self,
            url: &str,
            specs: &[ArtworkSpec],
        ) -> Result<Vec<ProcessedArtwork>, ArtworkError> {
            self.answer(format!("jpeg {url}"), self.jpeg, specs)
        }

        async fn process_bytes(
            &self,
            bytes: Vec<u8>,
            specs: &[ArtworkSpec],
        ) -> Result<Vec<ProcessedArtwork>, ArtworkError> {
            self.answer(format!("bytes {bytes:?}"), self.bytes, specs)
        }
    }

    #[derive(Clone, Default)]
    struct MockStills {
        fail: bool,
        calls: Arc<Mutex<Vec<(String, u64)>>>,
    }

    impl StillExtractor for MockStills {
        async fn extract(&self, input_path: &str, offset_ms: u64) -> Result<Vec<u8>, StillError> {
            self.calls.lock().unwrap().push((input_path.to_owned(), offset_ms));
            if self.fail {
                return Err(StillError::Backend("no video stream".into()));
            }
            Ok(vec![0xFF, 0xD8])
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
                sources: vec![ArtworkSource::Provider("https://cdn/poster.jpg".into())],
            }],
        }
    }

    const THUMBNAIL: &str = "https://i.ytimg.com/vi/abc/maxresdefault.jpg";
    const VIDEO: &str = "/ext/Show/Season 01/Show - S01E02 - Pilot.mkv";

    fn site_payload(sources: Vec<ArtworkSource>) -> ArtworkJobPayload {
        ArtworkJobPayload {
            owner: ArtworkOwner::Episode(EpisodeId("e1".into())),
            items: vec![ArtworkJobItem {
                id: ArtworkId("art-1".into()),
                kind: ArtworkKind::Backdrop,
                sources,
            }],
        }
    }

    fn thumbnail_then_frame() -> ArtworkJobPayload {
        site_payload(vec![
            ArtworkSource::Thumbnail(THUMBNAIL.into()),
            ArtworkSource::Frame { path: VIDEO.into(), offset_ms: 61_500 },
        ])
    }

    async fn episode_artwork(catalog: &MockCatalogRepo) -> Vec<ArtworkRef> {
        catalog.list_artwork(&ArtworkOwner::Episode(EpisodeId("e1".into()))).await.unwrap()
    }

    #[tokio::test]
    async fn the_first_source_that_renders_wins() {
        let catalog = MockCatalogRepo::new();
        let pipeline = MockPipeline::new(PipelineMode::Ok);
        let stills = MockStills::default();
        let handler = ArtworkJobHandler::new(
            catalog.clone(),
            pipeline.clone(),
            MockStore::default(),
            stills.clone(),
        );

        handler.handle(&job(thumbnail_then_frame().encode())).await.unwrap();

        assert_eq!(pipeline.calls(), vec![format!("jpeg {THUMBNAIL}")]);
        assert!(stills.calls.lock().unwrap().is_empty(), "no frame is taken once one renders");
        let refs = episode_artwork(&catalog).await;
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].kind, ArtworkKind::Backdrop);
    }

    #[tokio::test]
    async fn a_thumbnail_that_fails_falls_back_to_a_frame() {
        for failure in [PipelineMode::Download, PipelineMode::Decode] {
            let catalog = MockCatalogRepo::new();
            let pipeline = MockPipeline::new(PipelineMode::Ok).with_jpeg(failure);
            let stills = MockStills::default();
            let handler = ArtworkJobHandler::new(
                catalog.clone(),
                pipeline.clone(),
                MockStore::default(),
                stills.clone(),
            );

            handler.handle(&job(thumbnail_then_frame().encode())).await.unwrap();

            assert_eq!(
                pipeline.calls(),
                vec![format!("jpeg {THUMBNAIL}"), "bytes [255, 216]".to_owned()]
            );
            assert_eq!(*stills.calls.lock().unwrap(), vec![(VIDEO.to_owned(), 61_500)]);
            assert_eq!(episode_artwork(&catalog).await.len(), 1);
        }
    }

    #[tokio::test]
    async fn a_frame_that_cannot_be_taken_skips_the_item() {
        let catalog = MockCatalogRepo::new();
        let store = MockStore::default();
        let pipeline = MockPipeline::new(PipelineMode::Ok).with_jpeg(PipelineMode::Download);
        let handler = ArtworkJobHandler::new(
            catalog.clone(),
            pipeline.clone(),
            store.clone(),
            MockStills { fail: true, ..MockStills::default() },
        );

        handler.handle(&job(thumbnail_then_frame().encode())).await.unwrap();

        assert_eq!(pipeline.calls(), vec![format!("jpeg {THUMBNAIL}")]);
        assert!(store.stored.lock().unwrap().is_empty());
        assert!(episode_artwork(&catalog).await.is_empty());
    }

    #[tokio::test]
    async fn a_frame_that_cannot_be_decoded_skips_the_item() {
        let catalog = MockCatalogRepo::new();
        let handler = ArtworkJobHandler::new(
            catalog.clone(),
            MockPipeline::new(PipelineMode::Ok).with_bytes(PipelineMode::Decode),
            MockStore::default(),
            MockStills::default(),
        );

        let frame_only =
            site_payload(vec![ArtworkSource::Frame { path: VIDEO.into(), offset_ms: 0 }]);
        handler.handle(&job(frame_only.encode())).await.unwrap();

        assert!(episode_artwork(&catalog).await.is_empty());
    }

    async fn store_episode_art(catalog: &MockCatalogRepo) {
        let stored = ArtworkRef {
            id: ArtworkId("tmdb".into()),
            kind: ArtworkKind::Backdrop,
            widths: Vec::new(),
        };
        catalog
            .set_artwork(&ArtworkOwner::Episode(EpisodeId("e1".into())), &[stored])
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn a_fallback_finds_the_art_that_landed_before_it_and_leaves_it() {
        let catalog = MockCatalogRepo::new();
        store_episode_art(&catalog).await;
        let pipeline = MockPipeline::new(PipelineMode::Ok);
        let stills = MockStills::default();
        let handler = ArtworkJobHandler::new(
            catalog.clone(),
            pipeline.clone(),
            MockStore::default(),
            stills.clone(),
        );

        handler.handle(&job(thumbnail_then_frame().encode())).await.unwrap();

        assert!(pipeline.calls().is_empty());
        assert!(stills.calls.lock().unwrap().is_empty());
        let refs = episode_artwork(&catalog).await;
        assert_eq!(refs.iter().map(|art| art.id.0.as_str()).collect::<Vec<_>>(), ["tmdb"]);
    }

    struct ArtLandsWhileFraming(MockCatalogRepo);

    impl StillExtractor for ArtLandsWhileFraming {
        async fn extract(&self, _input_path: &str, _offset_ms: u64) -> Result<Vec<u8>, StillError> {
            store_episode_art(&self.0).await;
            Ok(vec![0xFF, 0xD8])
        }
    }

    #[tokio::test]
    async fn art_that_lands_while_the_fallback_renders_is_kept() {
        let catalog = MockCatalogRepo::new();
        let handler = ArtworkJobHandler::new(
            catalog.clone(),
            MockPipeline::new(PipelineMode::Ok),
            MockStore::default(),
            ArtLandsWhileFraming(catalog.clone()),
        );

        let frame_only =
            site_payload(vec![ArtworkSource::Frame { path: VIDEO.into(), offset_ms: 0 }]);
        handler.handle(&job(frame_only.encode())).await.unwrap();

        let refs = episode_artwork(&catalog).await;
        assert_eq!(refs.iter().map(|art| art.id.0.as_str()).collect::<Vec<_>>(), ["tmdb"]);
    }

    #[tokio::test]
    async fn provider_art_still_replaces_what_the_owner_has() {
        let catalog = MockCatalogRepo::new();
        store_episode_art(&catalog).await;
        let handler = ArtworkJobHandler::new(
            catalog.clone(),
            MockPipeline::new(PipelineMode::Ok),
            MockStore::default(),
            MockStills::default(),
        );

        let provider = site_payload(vec![ArtworkSource::Provider("https://cdn/still.jpg".into())]);
        handler.handle(&job(provider.encode())).await.unwrap();

        let refs = episode_artwork(&catalog).await;
        assert_eq!(refs.iter().map(|art| art.id.0.as_str()).collect::<Vec<_>>(), ["art-1"]);
    }

    #[tokio::test]
    async fn an_item_with_no_source_records_nothing() {
        let catalog = MockCatalogRepo::new();
        let pipeline = MockPipeline::new(PipelineMode::Ok);
        let handler = ArtworkJobHandler::new(
            catalog.clone(),
            pipeline.clone(),
            MockStore::default(),
            MockStills::default(),
        );

        handler.handle(&job(site_payload(Vec::new()).encode())).await.unwrap();

        assert!(pipeline.calls().is_empty());
        assert!(episode_artwork(&catalog).await.is_empty());
    }

    #[tokio::test]
    async fn a_payload_queued_before_sources_renders_its_url() {
        let catalog = MockCatalogRepo::new();
        let pipeline = MockPipeline::new(PipelineMode::Ok);
        let handler = ArtworkJobHandler::new(
            catalog.clone(),
            pipeline.clone(),
            MockStore::default(),
            MockStills::default(),
        );
        let raw = r#"{"owner":{"kind":"movie","id":"m1"},"items":[{"id":"art-1","kind":"poster","url":"https://cdn/poster.jpg"}]}"#;

        handler.handle(&job(raw.to_owned())).await.unwrap();

        assert_eq!(pipeline.calls(), vec!["url https://cdn/poster.jpg".to_owned()]);
        let refs = catalog.list_artwork(&ArtworkOwner::Movie(MovieId("m1".into()))).await.unwrap();
        assert_eq!(refs.len(), 1);
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
            MockStills::default(),
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
        let handler = ArtworkJobHandler::new(
            MockCatalogRepo::new(),
            pipeline.clone(),
            store.clone(),
            MockStills::default(),
        );

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
            sources: vec![ArtworkSource::Provider("https://cdn/logo.png".into())],
        });
        let handler = ArtworkJobHandler::new(
            catalog.clone(),
            MockPipeline::new(PipelineMode::Decode),
            store.clone(),
            MockStills::default(),
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
            MockStills::default(),
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
            MockStills::default(),
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
            MockStills::default(),
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
            MockStills::default(),
        );
        assert!(matches!(
            handler.handle(&job(payload().encode())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn a_fallback_that_cannot_read_the_owners_art_is_retried() {
        let catalog = MockCatalogRepo::new();
        catalog.set_fail();
        let stills = MockStills::default();
        let handler = ArtworkJobHandler::new(
            catalog,
            MockPipeline::new(PipelineMode::Ok),
            MockStore::default(),
            stills.clone(),
        );
        assert!(matches!(
            handler.handle(&job(thumbnail_then_frame().encode())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
        assert!(stills.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn invalid_payload_is_permanent() {
        let handler = ArtworkJobHandler::new(
            MockCatalogRepo::new(),
            MockPipeline::new(PipelineMode::Ok),
            MockStore::default(),
            MockStills::default(),
        );
        assert!(matches!(
            handler.handle(&job("garbage".into())).await.unwrap_err(),
            JobError::Permanent(_)
        ));
    }
}

use std::future::Future;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum_server::Handle;
use axum_server::tls_rustls::RustlsConfig;
use domain::job::{JobKind, JobPriority};
use domain::repository::JobRepository;
use fetch::{YtDlpFetcher, yt_dlp_version};
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};
use jobs::{
    ArtworkJobHandler, CacheEvictionHandler, CancelRegistry, CombineJobHandler,
    CompositeJobHandler, FetchJobHandler, IngestJobHandler, JobQueue, LibraryScanHandler,
    MetadataJobHandler, OrphanSweepHandler, RelinkJobHandler, RetentionHandler, RetryPolicy,
    Schedule, ScheduledScanHandler, Scheduler, SearchReindexHandler, SubtitlesJobHandler,
    TranscriptionJobHandler, TranslationJobHandler, TrickplayJobHandler, UpscaleJobHandler, Worker,
    enrichment_kinds, fetch_kinds, normal_kinds,
};
use media::artwork::FsArtworkStore;
use media::cache::CacheEvictor;
use media::download_token::HmacDownloadTokens;
use media::probe::FfprobeMediaProbe;
use media::scan::WalkdirSourceWalker;
use media::subtitle_store::FsSubtitleStore;
use media::transcode::VideoEncoder;
use media::trickplay::FfmpegTrickplayGenerator;
use media::upscale::FfmpegUpscaler;
use metadata::{
    CombiningProvider, ImageArtworkPipeline, OmdbClient, OpenSubtitlesClient, TmdbClient,
};
use metrics_exporter_prometheus::PrometheusHandle;
use persistence::job_log::FsJobLogStore;
use persistence::server::{SqliteCatalogRepo, SqliteJobRepo, SqliteLibraryRepo};
use services::library::{
    Enricher, LibraryServiceImpl, Scanner, TranscriptionEnqueuer, TranslationEnqueuer,
    next_daily_fire,
};
use services::user::UserServiceImpl;
use tokio::net::TcpListener;
use tokio::task::JoinSet;

use crate::api::{Built, Repos, SessionSvc, WireConfig, app, build_state};
use crate::bootstrap::{
    BootstrapResult, LibraryBootstrapProvider, UserBootstrapProvider, complete, run_one,
};
use crate::config::{Config, resolve_vaapi_device, transcription_model_dir, translation_model_dir};
use crate::lockfile::ServerLock;
use crate::observability::observability_router;

type ServerMetadataProvider = CombiningProvider<TmdbClient, OmdbClient>;
type ServerEnricher =
    Enricher<SqliteCatalogRepo, ServerMetadataProvider, SqliteJobRepo, SqliteLibraryRepo>;
type ScanHandler =
    LibraryScanHandler<SqliteLibraryRepo, WalkdirSourceWalker, FfprobeMediaProbe, ServerEnricher>;
type ReindexHandler = SearchReindexHandler<SqliteCatalogRepo>;
type ArtworkHandler = ArtworkJobHandler<SqliteCatalogRepo, ImageArtworkPipeline, FsArtworkStore>;
type TrickplayHandler = TrickplayJobHandler<SqliteCatalogRepo, FfmpegTrickplayGenerator>;
type IngestHandler = IngestJobHandler<SqliteLibraryRepo, FfprobeMediaProbe, ServerEnricher>;
type MetadataHandler = MetadataJobHandler<ServerEnricher>;
type RelinkHandler = RelinkJobHandler<SqliteLibraryRepo, FfprobeMediaProbe, ServerEnricher>;
type Trigger = TranslationEnqueuer<SqliteJobRepo>;
type TranscriptionTriggerImpl = TranscriptionEnqueuer<SqliteJobRepo, SqliteCatalogRepo>;
type SubtitlesHandler = SubtitlesJobHandler<
    OpenSubtitlesClient,
    SqliteCatalogRepo,
    FsSubtitleStore,
    Trigger,
    TranscriptionTriggerImpl,
>;
#[cfg(feature = "enrichment")]
type TranscriptionProviderImpl =
    inference::WhisperProvider<inference::Ct2WhisperEngine, media::transcode::TokioProcessSpawner>;
#[cfg(not(feature = "enrichment"))]
type TranscriptionProviderImpl = inference::DisabledTranscriptionProvider;
type TranscriptionHandler =
    TranscriptionJobHandler<TranscriptionProviderImpl, SqliteCatalogRepo, FsSubtitleStore, Trigger>;
#[cfg(feature = "enrichment")]
type TranslationProviderImpl = inference::MtProvider<inference::Ct2TranslationEngine>;
#[cfg(not(feature = "enrichment"))]
type TranslationProviderImpl = inference::DisabledTranslationProvider;
type TranslationHandler =
    TranslationJobHandler<TranslationProviderImpl, SqliteCatalogRepo, FsSubtitleStore>;
type UpscaleHandler = UpscaleJobHandler<FfmpegUpscaler, SqliteCatalogRepo, FfprobeMediaProbe>;
type CombineHandler =
    CombineJobHandler<inference::SubtitleMerger, SqliteCatalogRepo, FsSubtitleStore>;
type FetchHandler = FetchJobHandler<
    SqliteLibraryRepo,
    YtDlpFetcher<media::transcode::TokioProcessSpawner>,
    FfprobeMediaProbe,
    ServerEnricher,
>;
type EvictionHandler = CacheEvictionHandler<CacheEvictor>;
type NightlyHandler = ScheduledScanHandler<SqliteLibraryRepo, SqliteJobRepo>;
type RetentionJobHandler = RetentionHandler<SqliteJobRepo, FsJobLogStore>;
type SweepHandler = OrphanSweepHandler<
    SqliteCatalogRepo,
    FsArtworkStore,
    FsSubtitleStore,
    FfmpegTrickplayGenerator,
>;
type JobWorker = Worker<
    SqliteJobRepo,
    CompositeJobHandler<
        ScanHandler,
        ReindexHandler,
        ArtworkHandler,
        TrickplayHandler,
        IngestHandler,
        MetadataHandler,
        RelinkHandler,
        SubtitlesHandler,
        TranscriptionHandler,
        TranslationHandler,
        UpscaleHandler,
        CombineHandler,
        FetchHandler,
        EvictionHandler,
        NightlyHandler,
        RetentionJobHandler,
        SweepHandler,
    >,
    FsJobLogStore,
>;
type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[cfg(feature = "enrichment")]
fn select_model(
    kind: &str,
    base: &std::path::Path,
    enabled: bool,
    scan: impl Fn(&std::path::Path) -> inference::ModelScan,
) -> Result<Option<PathBuf>, BoxError> {
    if !enabled {
        return Ok(None);
    }
    let scan = scan(base);
    for rejected in &scan.rejected {
        tracing::warn!(
            "{kind}: ignoring model folder [{}]: {}",
            rejected.dir.display(),
            rejected.reason
        );
    }
    match scan.chosen {
        Some(model) => {
            tracing::info!(
                "{kind}: selected model [{}] from [{}] folder(s) under [{}]",
                model.display(),
                scan.total,
                base.display()
            );
            if let Some(warning) = crate::memory::memory_warning(
                kind,
                &model.display().to_string(),
                crate::memory::dir_size_bytes(&model),
                crate::memory::available_memory_bytes(),
            ) {
                tracing::warn!("{warning}");
            }
            Ok(Some(model))
        }
        None if scan.total > 0 => Err(format!(
            "{kind}: enabled but none of the [{}] folder(s) under [{}] contain a valid model; add a valid CTranslate2 model or disable {kind}",
            scan.total,
            base.display()
        )
        .into()),
        None => {
            tracing::warn!(
                "{kind}: enabled but no model folders were found under [{}]; {kind} jobs will fail until a model is added",
                base.display()
            );
            Ok(None)
        }
    }
}

fn build_transcription_provider(model_dir: PathBuf) -> TranscriptionProviderImpl {
    #[cfg(feature = "enrichment")]
    {
        inference::WhisperProvider::new(
            inference::Ct2WhisperEngine::new(model_dir),
            media::transcode::TokioProcessSpawner,
        )
    }
    #[cfg(not(feature = "enrichment"))]
    {
        let _ = model_dir;
        inference::DisabledTranscriptionProvider
    }
}

fn build_translation_provider(
    model_dir: PathBuf,
    source_prefix: Option<String>,
    target_prefix: Option<String>,
) -> TranslationProviderImpl {
    #[cfg(feature = "enrichment")]
    {
        inference::MtProvider::new(inference::Ct2TranslationEngine::new(
            model_dir,
            source_prefix,
            target_prefix,
        ))
    }
    #[cfg(not(feature = "enrichment"))]
    {
        let _ = (model_dir, source_prefix, target_prefix);
        inference::DisabledTranslationProvider
    }
}

async fn run_bootstrap(config: &Config, repos: &Repos) -> Result<BootstrapResult, BoxError> {
    let library_service = || {
        LibraryServiceImpl::new(
            repos.library.clone(),
            repos.users.clone(),
            repos.jobs.clone(),
            repos.catalog.clone(),
            config.tmdb_api_key.clone().map(|key| {
                TmdbClient::new(key)
                    .with_min_interval(Duration::from_millis(config.tmdb_min_interval_ms))
            }),
        )
    };
    let libraries = run_one(
        &LibraryBootstrapProvider::new(library_service()),
        &config.bootstrap_dir,
    )
    .await;
    let users = run_one(
        &UserBootstrapProvider::new(
            UserServiceImpl::new(
                repos.users.clone(),
                repos.auth_tokens.clone(),
                repos.user_data.clone(),
            ),
            library_service(),
        ),
        &config.bootstrap_dir,
    )
    .await;
    Ok(complete(libraries + users))
}

fn box_err<E: std::error::Error + Send + Sync + 'static>(error: E) -> BoxError {
    Box::new(error)
}

async fn resolve_yt_dlp_version(binary: &str) -> Option<String> {
    match yt_dlp_version(&media::transcode::TokioProcessSpawner, binary).await {
        Ok(version) => Some(version),
        Err(e) => {
            tracing::warn!("startup: yt-dlp at [{binary}] reported no version: {e}");
            None
        }
    }
}

async fn load_tls(
    cert: Option<&Path>,
    key: Option<&Path>,
) -> Result<Option<RustlsConfig>, BoxError> {
    match (cert, key) {
        (None, None) => Ok(None),
        (Some(cert), Some(key)) => Ok(Some(RustlsConfig::from_pem_file(cert, key).await?)),
        _ => Err("tls_cert and tls_key must both be set to enable TLS".into()),
    }
}

async fn stopped(mut rx: tokio::sync::watch::Receiver<bool>) {
    let _ = rx.wait_for(|stop| *stop).await;
}

fn join_outcome(
    joined: Result<Result<(), BoxError>, tokio::task::JoinError>,
) -> Result<(), BoxError> {
    match joined {
        Ok(inner) => inner,
        Err(join_error) => Err(Box::new(join_error)),
    }
}

async fn drain_within(timeout: Duration, drain: impl Future<Output = ()>) -> bool {
    tokio::time::timeout(timeout, drain).await.is_err()
}

pub struct Runtime {
    repos: Repos,
    router: Router,
    session: SessionSvc,
    worker: JobWorker,
    enrichment_worker: JobWorker,
    fetch_worker: JobWorker,
    scheduler: Scheduler,
    queue: JobQueue<SqliteJobRepo>,
    bind: SocketAddr,
    tls: Option<RustlsConfig>,
    worker_period: Duration,
    scheduler_period: Duration,
    reaper_period: Duration,
    shutdown_timeout: Duration,
    serve: bool,
}

impl Runtime {
    pub async fn build(config: Config, metrics: PrometheusHandle) -> Result<Self, BoxError> {
        let yt_dlp = match config.fetch_providers.enabled {
            true => resolve_yt_dlp_version(&config.fetch_providers.yt_dlp_binary).await,
            false => None,
        };
        tracing::info!("{}", config.describe(yt_dlp.as_deref()));
        let repos = Repos::connect(&config.db_root).await?;

        let reclaimed = repos
            .jobs
            .reclaim_running(Timestamp::now(), RetryPolicy::default().max_attempts)
            .await?;
        tracing::info!(
            requeued = reclaimed.requeued,
            dead_lettered = reclaimed.dead_lettered,
            "startup: reclaimed [{}] stale running jobs",
            reclaimed.total()
        );

        match CacheEvictor::new(&config.transcode_cache).purge().await {
            Ok(purged) => {
                tracing::info!("startup: purged [{purged}] stale transcode cache entries")
            }
            Err(error) => {
                tracing::warn!("startup: could not purge the transcode cache: [{error}]")
            }
        }

        let serve = config.bootstrap_mode.serves();
        if config.bootstrap_mode.enabled() {
            run_bootstrap(&config, &repos).await?;
        }

        let vaapi_present = config.vaapi_device.exists();
        let vaapi_device = resolve_vaapi_device(
            config.hardware_acceleration,
            &config.vaapi_device.to_string_lossy(),
            vaapi_present,
        );
        let download_secret = config.stream_secret.clone().into_bytes();
        let wire = WireConfig {
            jwt_secret: config.jwt_secret.into_bytes(),
            stream_secret: config.stream_secret.into_bytes(),
            access_ttl_secs: config.access_ttl_secs,
            refresh_ttl_secs: config.refresh_ttl_secs,
            transcode_cache: config.transcode_cache.clone(),
            artwork_cache: config.artwork_cache.clone(),
            trickplay_cache: config.trickplay_cache.clone(),
            tmdb_api_key: config.tmdb_api_key.clone(),
            transcription_enabled: config.enrichment.transcription.enabled,
            translation_enabled: config.enrichment.translation.enabled,
            upscaling_enabled: config.enrichment.upscaling.enabled,
            content_fetch_enabled: config.fetch_providers.enabled,
            fetch_cookies_file: config.fetch_providers.cookies_file.clone(),
            vaapi_device: vaapi_device.clone(),
            remux_read_rate: config.remux_read_rate,
            max_transcode_height: config.max_transcode_height,
            profile_overrides_dir: config.profile_overrides_dir.clone(),
        };
        let cancel = CancelRegistry::default();
        let Built {
            state,
            stream,
            session,
            artwork_store,
            images,
            trickplay,
        } = build_state(&repos, &wire, &cancel)?;
        let job_logs = FsJobLogStore::new(&config.job_log_dir);
        let transcription_dir = transcription_model_dir(&config.enrichment);
        let translation_dir = translation_model_dir(&config.enrichment);
        #[cfg(feature = "enrichment")]
        let (transcription_model, translation_model) = (
            select_model(
                "transcription",
                &transcription_dir,
                wire.transcription_enabled,
                inference::scan_transcription_models,
            )?,
            select_model(
                "translation",
                &translation_dir,
                wire.translation_enabled,
                inference::scan_translation_models,
            )?,
        );
        #[cfg(not(feature = "enrichment"))]
        let (transcription_model, translation_model): (Option<PathBuf>, Option<PathBuf>) =
            (None, None);
        let capabilities =
            crate::capabilities::server_capabilities(crate::capabilities::CapabilityInputs {
                transcription: wire.transcription_enabled && transcription_model.is_some(),
                translation: wire.translation_enabled && translation_model.is_some(),
                upscaling: wire.upscaling_enabled,
                opensubtitles: config.opensubtitles_api_key.is_some(),
                tmdb: config.tmdb_api_key.is_some(),
                tls: config.tls_cert.is_some() && config.tls_key.is_some(),
                webhooks: !config.webhook_clients.is_empty(),
                content_fetch: config.fetch_providers.enabled,
                hardware_transcode_available: vaapi_present,
                hardware_transcode_enabled: vaapi_device.is_some(),
            });
        let mut router = app(
            state.clone(),
            stream,
            images,
            trickplay,
            config.webhook_clients.clone(),
            capabilities,
        )
        .merge(observability_router(metrics, repos.clone()))
        .merge(::api::basic_ui_router(&config.basic_client_dir))
        .merge(::api::subtitle_router(
            state.clone(),
            ::api::SubtitleState::new(
                repos.catalog.clone(),
                FsSubtitleStore::new(&config.subtitle_cache),
            ),
        ))
        .merge(::api::job_log_router(
            state.clone(),
            ::api::JobLogState::new(job_logs.clone()),
        ))
        .merge(::api::download_router(
            state.clone(),
            ::api::DownloadState::new(
                state.clone(),
                repos.catalog.clone(),
                HmacDownloadTokens::new(&download_secret),
            ),
        ));
        if config.opensubtitles_api_key.is_some() {
            router = router.merge(::api::subtitle_search_router(
                state,
                ::api::SubtitleSearchState::new(
                    repos.catalog.clone(),
                    FsSubtitleStore::new(&config.subtitle_cache),
                    OpenSubtitlesClient::new(
                        config.opensubtitles_api_key.clone().unwrap_or_default(),
                    )
                    .with_min_interval(Duration::from_millis(config.opensubtitles_min_interval_ms)),
                ),
            ));
        }
        if !config.cors_allowed_origins.is_empty() {
            router = router.layer(::api::cors_layer(&config.cors_allowed_origins));
        }

        let subtitle_langs = if config.opensubtitles_api_key.is_some() {
            config.target_languages.clone()
        } else {
            Vec::new()
        };
        let translation_langs = if config.enrichment.translation.enabled {
            config.target_languages.clone()
        } else {
            Vec::new()
        };
        let subtitles = SubtitlesJobHandler::new(
            OpenSubtitlesClient::new(config.opensubtitles_api_key.clone().unwrap_or_default())
                .with_min_interval(Duration::from_millis(config.opensubtitles_min_interval_ms)),
            repos.catalog.clone(),
            FsSubtitleStore::new(&config.subtitle_cache),
            TranslationEnqueuer::new(repos.jobs.clone(), translation_langs.clone()),
            TranscriptionEnqueuer::new(
                repos.jobs.clone(),
                repos.catalog.clone(),
                subtitle_langs.clone(),
            ),
        );
        let transcription_enabled = config.enrichment.transcription.enabled;
        let transcription = TranscriptionJobHandler::new(
            build_transcription_provider(
                transcription_model.unwrap_or_else(|| transcription_dir.clone()),
            ),
            repos.catalog.clone(),
            FsSubtitleStore::new(&config.subtitle_cache),
            TranslationEnqueuer::new(repos.jobs.clone(), translation_langs.clone()),
        );
        let translation = TranslationJobHandler::new(
            build_translation_provider(
                translation_model.unwrap_or_else(|| translation_dir.clone()),
                config.enrichment.translation.source_prefix.clone(),
                config.enrichment.translation.target_prefix.clone(),
            ),
            repos.catalog.clone(),
            FsSubtitleStore::new(&config.subtitle_cache),
        );
        let upscale = UpscaleJobHandler::new(
            FfmpegUpscaler::new().with_encoder(VideoEncoder::from_device(vaapi_device.clone())),
            repos.catalog.clone(),
            FfprobeMediaProbe::default(),
        );
        let combine = CombineJobHandler::new(
            inference::SubtitleMerger,
            repos.catalog.clone(),
            FsSubtitleStore::new(&config.subtitle_cache),
        );
        let tmdb_interval = Duration::from_millis(config.tmdb_min_interval_ms);
        let omdb_interval = Duration::from_millis(config.omdb_min_interval_ms);
        let omdb = config
            .omdb_api_key
            .map(|key| OmdbClient::new(key).with_min_interval(omdb_interval));
        let provider = config.tmdb_api_key.map(|key| {
            CombiningProvider::new(TmdbClient::new(key).with_min_interval(tmdb_interval), omdb)
        });
        let scan_enricher = Enricher::new(
            repos.catalog.clone(),
            provider.clone(),
            repos.jobs.clone(),
            repos.library.clone(),
        )
        .with_subtitle_languages(subtitle_langs.clone())
        .with_transcription(transcription_enabled)
        .with_translation_languages(translation_langs.clone());
        let ingest_enricher = Enricher::new(
            repos.catalog.clone(),
            provider.clone(),
            repos.jobs.clone(),
            repos.library.clone(),
        )
        .with_subtitle_languages(subtitle_langs.clone())
        .with_transcription(transcription_enabled)
        .with_translation_languages(translation_langs.clone());
        let fetch_enricher = Enricher::new(
            repos.catalog.clone(),
            provider.clone(),
            repos.jobs.clone(),
            repos.library.clone(),
        )
        .with_subtitle_languages(subtitle_langs.clone())
        .with_transcription(transcription_enabled)
        .with_translation_languages(translation_langs.clone());
        let relink_enricher = Enricher::new(
            repos.catalog.clone(),
            provider.clone(),
            repos.jobs.clone(),
            repos.library.clone(),
        )
        .with_subtitle_languages(subtitle_langs)
        .with_transcription(transcription_enabled)
        .with_translation_languages(translation_langs.clone());
        let metadata_enricher = Enricher::new(
            repos.catalog.clone(),
            provider,
            repos.jobs.clone(),
            repos.library.clone(),
        );
        let scan = LibraryScanHandler::new(
            repos.library.clone(),
            Scanner::new(WalkdirSourceWalker, FfprobeMediaProbe::default())
                .with_probe_concurrency(config.scan_probe_concurrency),
            scan_enricher,
        );
        let reindex = SearchReindexHandler::new(repos.catalog.clone());
        let artwork = ArtworkJobHandler::new(
            repos.catalog.clone(),
            ImageArtworkPipeline::new(),
            artwork_store,
        );
        let trickplay = TrickplayJobHandler::new(
            repos.catalog.clone(),
            FfmpegTrickplayGenerator::new(&config.trickplay_cache),
        );
        let ingest = IngestJobHandler::new(
            repos.library.clone(),
            FfprobeMediaProbe::default(),
            ingest_enricher,
        );
        let metadata = MetadataJobHandler::new(metadata_enricher);
        let relink = RelinkJobHandler::new(
            repos.library.clone(),
            FfprobeMediaProbe::default(),
            relink_enricher,
        );
        let fetch = FetchJobHandler::new(
            repos.library.clone(),
            YtDlpFetcher::new(
                media::transcode::TokioProcessSpawner,
                config.fetch_providers.yt_dlp_binary.clone(),
                config.fetch_providers.yt_dlp_plugin_dir.clone(),
                config.fetch_providers.max_height,
                config.fetch_providers.cookies_file.clone(),
            ),
            FfprobeMediaProbe::default(),
            fetch_enricher,
        );
        let eviction = CacheEvictionHandler::new(
            CacheEvictor::new(&config.transcode_cache),
            config.transcode_cache_cap_bytes,
        );
        let nightly = ScheduledScanHandler::new(repos.library.clone(), repos.jobs.clone());
        let retention = RetentionHandler::new(
            repos.jobs.clone(),
            job_logs.clone(),
            SignedDuration::from_hours(config.job_retention_days.saturating_mul(24)),
        );
        let sweep = OrphanSweepHandler::new(
            repos.catalog.clone(),
            FsArtworkStore::new(&config.artwork_cache),
            FsSubtitleStore::new(&config.subtitle_cache),
            FfmpegTrickplayGenerator::new(&config.trickplay_cache),
            SignedDuration::from_secs(config.orphan_sweep_grace_secs),
        );
        let handler = Arc::new(CompositeJobHandler::new(
            scan,
            reindex,
            artwork,
            trickplay,
            ingest,
            metadata,
            relink,
            subtitles,
            transcription,
            translation,
            upscale,
            combine,
            fetch,
            eviction,
            nightly,
            retention,
            sweep,
        ));
        let worker = Worker::new(
            repos.jobs.clone(),
            handler.clone(),
            job_logs.clone(),
            config.worker_concurrency,
            RetryPolicy::default(),
            normal_kinds(),
        )
        .with_cancel(cancel.clone());
        let enrichment_worker = Worker::new(
            repos.jobs.clone(),
            handler.clone(),
            job_logs.clone(),
            config.enrichment.concurrency,
            RetryPolicy::default(),
            enrichment_kinds(),
        )
        .with_cancel(cancel.clone());
        let fetch_worker = Worker::new(
            repos.jobs.clone(),
            handler,
            job_logs,
            config.fetch_providers.concurrency,
            RetryPolicy::default(),
            fetch_kinds(),
        )
        .with_cancel(cancel);

        let tls = load_tls(config.tls_cert.as_deref(), config.tls_key.as_deref()).await?;

        let queue = JobQueue::new(repos.jobs.clone());
        let mut scheduler = Scheduler::new();
        let now = Timestamp::now();
        let reindex_every = SignedDuration::from_secs(config.reindex_every_secs);
        scheduler.register(Schedule {
            kind: JobKind::SearchReindex,
            priority: JobPriority::Normal,
            payload: String::new(),
            every: reindex_every,
            next_fire_at: now.saturating_add(reindex_every).unwrap_or(now),
        });
        let cache_evict_every = SignedDuration::from_secs(config.cache_eviction_every_secs);
        scheduler.register(Schedule {
            kind: JobKind::CacheEviction,
            priority: JobPriority::Normal,
            payload: String::new(),
            every: cache_evict_every,
            next_fire_at: now.saturating_add(cache_evict_every).unwrap_or(now),
        });
        let retention_every = SignedDuration::from_secs(config.retention_every_secs);
        scheduler.register(Schedule {
            kind: JobKind::Retention,
            priority: JobPriority::Low,
            payload: String::new(),
            every: retention_every,
            next_fire_at: now.saturating_add(retention_every).unwrap_or(now),
        });
        let sweep_every = SignedDuration::from_secs(config.orphan_sweep_every_secs);
        scheduler.register(Schedule {
            kind: JobKind::OrphanSweep,
            priority: JobPriority::Low,
            payload: String::new(),
            every: sweep_every,
            next_fire_at: now.saturating_add(sweep_every).unwrap_or(now),
        });
        if let Some(at) = config.daily_scan_at.as_deref() {
            match next_daily_fire(at, now, TimeZone::system()) {
                Some(next_fire_at) => scheduler.register(Schedule {
                    kind: JobKind::ScheduledScan,
                    priority: JobPriority::Normal,
                    payload: String::new(),
                    every: SignedDuration::from_hours(24),
                    next_fire_at,
                }),
                None => {
                    tracing::warn!(
                        "ignoring daily_scan_at [{at}], expected a clock time like 04:00"
                    )
                }
            }
        }

        Ok(Self {
            repos,
            router,
            session,
            worker,
            enrichment_worker,
            fetch_worker,
            scheduler,
            queue,
            bind: config.bind,
            tls,
            worker_period: Duration::from_secs(config.worker_period_secs),
            scheduler_period: Duration::from_secs(config.scheduler_period_secs),
            reaper_period: Duration::from_secs(config.reaper_period_secs),
            shutdown_timeout: Duration::from_secs(config.shutdown_timeout_secs),
            serve,
        })
    }

    pub async fn run(self, shutdown: impl Future<Output = ()>) -> Result<(), BoxError> {
        if !self.serve {
            self.repos.close().await;
            return Ok(());
        }
        let listener = TcpListener::bind(self.bind).await?;
        let Runtime {
            repos,
            router,
            session,
            worker,
            enrichment_worker,
            fetch_worker,
            mut scheduler,
            queue,
            tls,
            worker_period,
            scheduler_period,
            reaper_period,
            shutdown_timeout,
            bind: _,
            serve: _,
        } = self;

        let (stop_tx, _stop_rx) = tokio::sync::watch::channel(false);
        let mut tasks: JoinSet<Result<(), BoxError>> = JoinSet::new();

        let worker_stop = stopped(stop_tx.subscribe());
        tasks.spawn(async move {
            worker
                .run(worker_period, worker_stop)
                .await
                .map_err(box_err)
        });

        let enrichment_stop = stopped(stop_tx.subscribe());
        tasks.spawn(async move {
            enrichment_worker
                .run(worker_period, enrichment_stop)
                .await
                .map_err(box_err)
        });

        let fetch_stop = stopped(stop_tx.subscribe());
        tasks.spawn(async move {
            fetch_worker
                .run(worker_period, fetch_stop)
                .await
                .map_err(box_err)
        });

        let scheduler_stop = stopped(stop_tx.subscribe());
        tasks.spawn(async move {
            scheduler
                .run(scheduler_period, &queue, scheduler_stop)
                .await
                .map_err(box_err)
        });

        let reaper_stop = stopped(stop_tx.subscribe());
        tasks.spawn(async move {
            let mut ticker = tokio::time::interval(reaper_period);
            tokio::pin!(reaper_stop);
            loop {
                tokio::select! {
                    biased;
                    () = &mut reaper_stop => break,
                    _ = ticker.tick() => {
                        session.reap_idle().await;
                    }
                }
            }
            Ok(())
        });

        let server_stop = stopped(stop_tx.subscribe());
        match tls {
            None => {
                tasks.spawn(async move {
                    axum::serve(listener, router)
                        .with_graceful_shutdown(server_stop)
                        .await
                        .map_err(box_err)
                });
            }
            Some(tls) => {
                let handle = Handle::new();
                let shutdown_handle = handle.clone();
                tasks.spawn(async move {
                    server_stop.await;
                    shutdown_handle.graceful_shutdown(None);
                    Ok(())
                });
                let std_listener = listener.into_std()?;
                tasks.spawn(async move {
                    axum_server::from_tcp_rustls(std_listener, tls)?
                        .handle(handle)
                        .serve(router.into_make_service())
                        .await
                        .map_err(box_err)
                });
            }
        }

        tokio::pin!(shutdown);
        let mut result: Result<(), BoxError> = Ok(());
        tokio::select! {
            biased;
            () = &mut shutdown => {}
            Some(joined) = tasks.join_next() => {
                result = join_outcome(joined);
            }
        }

        let _ = stop_tx.send(true);

        let drain = async {
            while let Some(joined) = tasks.join_next().await {
                let outcome = join_outcome(joined);
                if result.is_ok() {
                    result = outcome;
                }
            }
        };
        let timed_out = drain_within(shutdown_timeout, drain).await;
        tasks.abort_all();
        tracing::info!(timed_out, "shutdown: background tasks drained");

        repos.close().await;
        result
    }
}

pub async fn serve(
    config: Config,
    metrics: PrometheusHandle,
    shutdown: impl Future<Output = ()>,
) -> Result<(), BoxError> {
    let db_root = config.db_root.clone();
    let Some(_lock) = ServerLock::try_acquire(&db_root)? else {
        return Err(format!("another shadowmask instance is using {}", db_root.display()).into());
    };
    let runtime = Runtime::build(config, metrics).await?;
    runtime.run(shutdown).await
}

#[cfg(test)]
mod tests {
    use std::future::pending;
    use std::path::Path;

    use domain::common::PageRequest;
    use domain::service::{LibraryService, UserService};
    use domain::user::Role;

    use super::*;
    use crate::api::LibrarySvc;
    use crate::bootstrap::{BootstrapMode, bootstrap_admin};

    fn write_bootstrap(dir: &Path) {
        std::fs::write(
            dir.join("libraries.toml"),
            "[[libraries]]\nname = \"Movies\"\nkind = \"movie\"\nroots = [\"/media/movies\"]\nwatcher = \"local\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("users.toml"),
            "[[users]]\nusername = \"admin\"\npassword = \"secret\"\nrole = \"admin\"\nlibraries = [\"Movies\"]\n",
        )
        .unwrap();
    }

    fn bootstrap_config(db_root: &Path, bootstrap_dir: &Path, mode: BootstrapMode) -> Config {
        Config {
            job_log_dir: db_root.join("job-logs"),
            db_root: db_root.to_owned(),
            bind: SocketAddr::from(([127, 0, 0, 1], 0)),
            bootstrap_mode: mode,
            bootstrap_dir: bootstrap_dir.to_owned(),
            ..Config::default()
        }
    }

    fn library_service(repos: &Repos) -> LibrarySvc {
        LibraryServiceImpl::new(
            repos.library.clone(),
            repos.users.clone(),
            repos.jobs.clone(),
            repos.catalog.clone(),
            None,
        )
    }

    fn test_config(db_root: PathBuf) -> Config {
        Config {
            job_log_dir: db_root.join("job-logs"),
            transcode_cache: db_root.join("transcode"),
            db_root,
            bind: SocketAddr::from(([127, 0, 0, 1], 0)),
            ..Config::default()
        }
    }

    fn test_metrics() -> PrometheusHandle {
        metrics_exporter_prometheus::PrometheusBuilder::new()
            .build_recorder()
            .handle()
    }

    #[tokio::test]
    async fn build_wires_a_working_worker() {
        let dir = tempfile::tempdir().unwrap();
        let runtime = Runtime::build(test_config(dir.path().to_owned()), test_metrics())
            .await
            .unwrap();
        assert_eq!(runtime.worker.run_once(Timestamp::now()).await.unwrap(), 0);
        assert_eq!(runtime.session.reap_idle().await, 0);
    }

    // Every session named in the cache died with the process that wrote it, so a
    // restart starts from an empty cache rather than waiting for the cap.
    #[tokio::test]
    async fn build_purges_the_transcode_cache_it_inherited() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path().to_owned());
        let cache = config.transcode_cache.clone();
        let stale = cache.join("session").join("1");
        std::fs::create_dir_all(&stale).unwrap();
        std::fs::write(stale.join("seg_00000.ts"), b"x").unwrap();

        Runtime::build(config, test_metrics()).await.unwrap();

        assert!(!cache.join("session").exists());
    }

    // The default config leaves every optional component switched off, so the
    // branches that wire the metadata providers, the CORS layer, the translation
    // languages and the daily scan are never taken by any other test.
    #[tokio::test]
    async fn a_fully_configured_build_wires_every_optional_component() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = test_config(dir.path().to_owned());
        config.tmdb_api_key = Some("tmdb-key".into());
        config.omdb_api_key = Some("omdb-key".into());
        config.opensubtitles_api_key = Some("os-key".into());
        config.target_languages = vec!["nl".into()];
        config.enrichment.translation.enabled = true;
        config.cors_allowed_origins = vec!["https://example.test".into()];
        config.daily_scan_at = Some("04:00".into());
        let profiles = dir.path().join("profiles");
        std::fs::create_dir_all(&profiles).unwrap();
        std::fs::write(
            profiles.join("lounge-tv.json"),
            r#"{"containers":["mp4"],"video":[{"codec":"h264","max_bit_depth":8}],"audio":[{"codec":"aac","max_channels":2}],"max_width":1920,"max_height":1080,"max_bitrate":8000000}"#,
        )
        .unwrap();
        config.profile_overrides_dir = Some(profiles);

        let runtime = Runtime::build(config, test_metrics()).await.unwrap();

        assert_eq!(runtime.worker.run_once(Timestamp::now()).await.unwrap(), 0);
    }

    // An operator who points at a directory that is not there has to hear about
    // it, since the alternative is reading a profile they think they replaced.
    #[tokio::test]
    async fn a_profile_override_directory_that_is_not_there_stops_the_build() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = test_config(dir.path().to_owned());
        config.profile_overrides_dir = Some(dir.path().join("absent"));

        assert!(Runtime::build(config, test_metrics()).await.is_err());
    }

    // A malformed clock time must be ignored rather than refusing to start, which
    // is the difference between a warning and an unbootable server.
    #[tokio::test]
    async fn a_daily_scan_time_that_is_not_a_clock_time_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = test_config(dir.path().to_owned());
        config.daily_scan_at = Some("half past four".into());

        assert!(Runtime::build(config, test_metrics()).await.is_ok());
    }

    #[tokio::test]
    async fn build_with_opensubtitles_key_wires_subtitle_fetch() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            opensubtitles_api_key: Some("os-key".to_owned()),
            target_languages: vec!["en".to_owned(), "es".to_owned()],
            ..test_config(dir.path().to_owned())
        };
        let runtime = Runtime::build(config, test_metrics()).await.unwrap();
        assert_eq!(runtime.worker.run_once(Timestamp::now()).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn a_fetch_enabled_server_reports_its_yt_dlp_at_startup() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = test_config(dir.path().to_owned());
        config.fetch_providers.enabled = true;
        // echo answers --version with the flag itself, which is all this needs:
        // a binary that runs and prints something.
        config.fetch_providers.yt_dlp_binary = "/bin/echo".to_owned();
        let runtime = Runtime::build(config, test_metrics()).await.unwrap();
        assert_eq!(runtime.worker.run_once(Timestamp::now()).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn a_missing_yt_dlp_warns_instead_of_stopping_the_server() {
        // Content fetch is one optional feature; a server whose yt-dlp is gone
        // still has to boot and serve everything else.
        assert_eq!(
            resolve_yt_dlp_version("/definitely/not/a/binary").await,
            None
        );
    }

    #[tokio::test]
    async fn run_drives_background_tasks_until_shutdown() {
        let dir = tempfile::tempdir().unwrap();
        let mut runtime = Runtime::build(test_config(dir.path().to_owned()), test_metrics())
            .await
            .unwrap();
        runtime.worker_period = Duration::from_millis(5);
        runtime.scheduler_period = Duration::from_millis(5);
        runtime.reaper_period = Duration::from_millis(5);

        runtime
            .run(async {
                tokio::time::sleep(Duration::from_millis(30)).await;
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn serve_runs_until_shutdown_and_releases_lock() {
        let dir = tempfile::tempdir().unwrap();
        serve(test_config(dir.path().to_owned()), test_metrics(), async {
            tokio::time::sleep(Duration::from_millis(20)).await;
        })
        .await
        .unwrap();
        assert!(ServerLock::try_acquire(dir.path()).unwrap().is_some());
    }

    #[tokio::test]
    async fn serve_refuses_when_db_root_locked() {
        let dir = tempfile::tempdir().unwrap();
        let held = ServerLock::try_acquire(dir.path()).unwrap();
        assert!(held.is_some());
        let result = serve(
            test_config(dir.path().to_owned()),
            test_metrics(),
            pending::<()>(),
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn drain_within_reports_completion() {
        assert!(!drain_within(Duration::from_secs(10), async {}).await);
    }

    #[tokio::test]
    async fn drain_within_reports_timeout() {
        assert!(drain_within(Duration::from_millis(5), pending::<()>()).await);
    }

    #[tokio::test]
    async fn join_outcome_maps_join_error() {
        let mut set: JoinSet<Result<(), BoxError>> = JoinSet::new();
        set.spawn(async {
            pending::<()>().await;
            Ok(())
        });
        set.abort_all();
        let joined = set.join_next().await.unwrap();
        assert!(join_outcome(joined).is_err());
    }

    #[tokio::test]
    async fn run_surfaces_background_task_error() {
        let dir = tempfile::tempdir().unwrap();
        let mut runtime = Runtime::build(test_config(dir.path().to_owned()), test_metrics())
            .await
            .unwrap();
        runtime.worker_period = Duration::from_millis(1);
        runtime.repos.jobs.close().await;
        let result = runtime.run(pending::<()>()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn run_bootstrap_creates_admin_library_and_access() {
        let db = tempfile::tempdir().unwrap();
        let boot = tempfile::tempdir().unwrap();
        write_bootstrap(boot.path());
        let repos = Repos::connect(db.path()).await.unwrap();
        let config = bootstrap_config(db.path(), boot.path(), BootstrapMode::InitAndStart);

        let result = run_bootstrap(&config, &repos).await.unwrap();
        assert_eq!(result.created, 2);
        assert_eq!(result.skipped, 0);

        let libraries = library_service(&repos)
            .libraries(&bootstrap_admin())
            .await
            .unwrap();
        assert_eq!(libraries.len(), 1);
        assert_eq!(libraries[0].name, "Movies");

        let users = UserServiceImpl::new(
            repos.users.clone(),
            repos.auth_tokens.clone(),
            repos.user_data.clone(),
        );
        let listed = users
            .list(PageRequest {
                offset: 0,
                limit: 10,
            })
            .await
            .unwrap();
        assert_eq!(listed.total, 1);
        let admin = &listed.items[0];
        assert_eq!(admin.username, "admin");
        assert_eq!(admin.role, Role::Admin);
        let access = users.library_access(&admin.id).await.unwrap();
        assert_eq!(access.len(), 1);
        assert_eq!(access[0].library, libraries[0].id);

        repos.close().await;
    }

    #[tokio::test]
    async fn run_bootstrap_is_idempotent() {
        let db = tempfile::tempdir().unwrap();
        let boot = tempfile::tempdir().unwrap();
        write_bootstrap(boot.path());
        let repos = Repos::connect(db.path()).await.unwrap();
        let config = bootstrap_config(db.path(), boot.path(), BootstrapMode::InitAndStart);

        let first = run_bootstrap(&config, &repos).await.unwrap();
        assert_eq!(first.created, 2);
        let second = run_bootstrap(&config, &repos).await.unwrap();
        assert_eq!(second.created, 0);
        assert_eq!(second.skipped, 2);

        let users = UserServiceImpl::new(
            repos.users.clone(),
            repos.auth_tokens.clone(),
            repos.user_data.clone(),
        );
        assert_eq!(
            users
                .list(PageRequest {
                    offset: 0,
                    limit: 10,
                })
                .await
                .unwrap()
                .total,
            1
        );
        repos.close().await;
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn run_bootstrap_expands_env_var() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("SHADOWMASK_ADMIN_PASSWORD", "hunter2");
            let db = tempfile::tempdir().unwrap();
            let boot = tempfile::tempdir().unwrap();
            std::fs::write(
                boot.path().join("users.toml"),
                "[[users]]\nusername = \"admin\"\npassword = \"${SHADOWMASK_ADMIN_PASSWORD}\"\nrole = \"admin\"\n",
            )
            .unwrap();
            let config = bootstrap_config(db.path(), boot.path(), BootstrapMode::Init);
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let repos = Repos::connect(db.path()).await.unwrap();
                let result = run_bootstrap(&config, &repos).await.unwrap();
                assert_eq!(result.created, 1);
                let users = UserServiceImpl::new(
                    repos.users.clone(),
                    repos.auth_tokens.clone(),
                    repos.user_data.clone(),
                );
                let listed = users
                    .list(PageRequest {
                        offset: 0,
                        limit: 10,
                    })
                    .await
                    .unwrap();
                assert_eq!(listed.items[0].username, "admin");
                repos.close().await;
            });
            Ok(())
        });
    }

    #[tokio::test]
    async fn build_with_init_bootstraps_but_does_not_serve() {
        let db = tempfile::tempdir().unwrap();
        let boot = tempfile::tempdir().unwrap();
        write_bootstrap(boot.path());
        let runtime = Runtime::build(
            bootstrap_config(db.path(), boot.path(), BootstrapMode::Init),
            test_metrics(),
        )
        .await
        .unwrap();
        assert!(!runtime.serve);
        let users = UserServiceImpl::new(
            runtime.repos.users.clone(),
            runtime.repos.auth_tokens.clone(),
            runtime.repos.user_data.clone(),
        );
        assert_eq!(
            users
                .list(PageRequest {
                    offset: 0,
                    limit: 10,
                })
                .await
                .unwrap()
                .total,
            1
        );
        runtime.run(pending::<()>()).await.unwrap();
    }

    #[tokio::test]
    async fn build_with_off_creates_nothing() {
        let db = tempfile::tempdir().unwrap();
        let boot = tempfile::tempdir().unwrap();
        write_bootstrap(boot.path());
        let runtime = Runtime::build(
            bootstrap_config(db.path(), boot.path(), BootstrapMode::Off),
            test_metrics(),
        )
        .await
        .unwrap();
        assert!(runtime.serve);
        let users = UserServiceImpl::new(
            runtime.repos.users.clone(),
            runtime.repos.auth_tokens.clone(),
            runtime.repos.user_data.clone(),
        );
        assert_eq!(
            users
                .list(PageRequest {
                    offset: 0,
                    limit: 10,
                })
                .await
                .unwrap()
                .total,
            0
        );
        assert!(
            library_service(&runtime.repos)
                .libraries(&bootstrap_admin())
                .await
                .unwrap()
                .is_empty()
        );
        runtime.repos.close().await;
    }

    const TEST_TLS_CERT: &str = "-----BEGIN CERTIFICATE-----
MIIBfTCCASOgAwIBAgIUVnWXLlTIJLumhg3a5tLxLroHoDwwCgYIKoZIzj0EAwIw
FDESMBAGA1UEAwwJbG9jYWxob3N0MB4XDTI2MDcyMDA2MjkxOFoXDTM2MDcxNzA2
MjkxOFowFDESMBAGA1UEAwwJbG9jYWxob3N0MFkwEwYHKoZIzj0CAQYIKoZIzj0D
AQcDQgAEvb0f4KmdKWzZowUNgB/2IlLpLRC27TiOyQdgYamRznrFK0x+9FdISCs/
niEeHSY+lbd8YxNr/az+7r/AdgerdqNTMFEwHQYDVR0OBBYEFFd2vzFTw6wWU+Eu
Z9WO+Last5r8MB8GA1UdIwQYMBaAFFd2vzFTw6wWU+EuZ9WO+Last5r8MA8GA1Ud
EwEB/wQFMAMBAf8wCgYIKoZIzj0EAwIDSAAwRQIgSulEcXgBcr0zOlCIg27ris1x
klqQalMiznoYYwi7Q+oCIQCbBBHJd1bAjQKhEt19y1BGyv7q7F+jIrx1Ly4b1r/M
LQ==
-----END CERTIFICATE-----
";

    const TEST_TLS_KEY: &str = "-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgDUNHjgk3jCHMpdH/
v7Jf+afqKgCd7OspjYoqPo0beP6hRANCAAS9vR/gqZ0pbNmjBQ2AH/YiUuktELbt
OI7JB2BhqZHOesUrTH70V0hIKz+eIR4dJj6Vt3xjE2v9rP7uv8B2B6t2
-----END PRIVATE KEY-----
";

    fn write_tls_pair(dir: &Path) -> (PathBuf, PathBuf) {
        let cert = dir.join("cert.pem");
        let key = dir.join("key.pem");
        std::fs::write(&cert, TEST_TLS_CERT).unwrap();
        std::fs::write(&key, TEST_TLS_KEY).unwrap();
        (cert, key)
    }

    #[tokio::test]
    async fn load_tls_none_when_both_unset() {
        assert!(load_tls(None, None).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn load_tls_requires_both_paths() {
        assert!(load_tls(Some(Path::new("cert.pem")), None).await.is_err());
        assert!(load_tls(None, Some(Path::new("key.pem"))).await.is_err());
    }

    #[tokio::test]
    async fn load_tls_errors_on_missing_files() {
        let dir = tempfile::tempdir().unwrap();
        let result = load_tls(
            Some(&dir.path().join("absent-cert.pem")),
            Some(&dir.path().join("absent-key.pem")),
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn load_tls_loads_a_pem_pair() {
        let dir = tempfile::tempdir().unwrap();
        let (cert, key) = write_tls_pair(dir.path());
        assert!(load_tls(Some(&cert), Some(&key)).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn serve_over_tls_until_shutdown() {
        let dir = tempfile::tempdir().unwrap();
        let (cert, key) = write_tls_pair(dir.path());
        let mut config = test_config(dir.path().to_owned());
        config.tls_cert = Some(cert);
        config.tls_key = Some(key);
        let mut runtime = Runtime::build(config, test_metrics()).await.unwrap();
        assert!(runtime.tls.is_some());
        runtime.worker_period = Duration::from_millis(5);
        runtime.scheduler_period = Duration::from_millis(5);
        runtime.reaper_period = Duration::from_millis(5);
        runtime
            .run(async {
                tokio::time::sleep(Duration::from_millis(30)).await;
            })
            .await
            .unwrap();
    }
}

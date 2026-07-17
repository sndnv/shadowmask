use std::future::Future;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use axum::Router;
use axum_server::Handle;
use axum_server::tls_rustls::RustlsConfig;
use domain::job::{JobKind, JobPriority};
use domain::repository::JobRepository;
use figment::Figment;
use figment::providers::{Env, Format, Serialized, Toml};
use jiff::{SignedDuration, Timestamp};
use jobs::{
    ArtworkJobHandler, CompositeJobHandler, IngestJobHandler, JobQueue, LibraryScanHandler,
    MetadataJobHandler, RelinkJobHandler, RetryPolicy, Schedule, Scheduler, SearchReindexHandler,
    SubtitlesJobHandler, TrickplayJobHandler, Worker,
};
use media::artwork::FsArtworkStore;
use media::probe::FfprobeMediaProbe;
use media::scan::WalkdirSourceWalker;
use media::subtitle_store::FsSubtitleStore;
use media::trickplay::FfmpegTrickplayGenerator;
use metadata::{ImageArtworkPipeline, OpenSubtitlesClient, TmdbClient};
use metrics_exporter_prometheus::PrometheusHandle;
use persistence::job_log::FsJobLogStore;
use persistence::server::{SqliteCatalogRepo, SqliteJobRepo, SqliteLibraryRepo};
use serde::{Deserialize, Serialize};
use services::library::{Enricher, LibraryServiceImpl, Scanner};
use services::user::UserServiceImpl;
use tokio::net::TcpListener;
use tokio::task::JoinSet;

use crate::api::{Built, Repos, SessionSvc, WireConfig, app, build_state};
use crate::bootstrap::{
    BootstrapMode, BootstrapResult, ErasedProvider, LibraryBootstrapProvider,
    UserBootstrapProvider, run_providers,
};
use crate::lockfile::ServerLock;
use crate::observability::observability_router;

type ServerEnricher = Enricher<SqliteCatalogRepo, TmdbClient, SqliteJobRepo, SqliteLibraryRepo>;
type ScanHandler =
    LibraryScanHandler<SqliteLibraryRepo, WalkdirSourceWalker, FfprobeMediaProbe, ServerEnricher>;
type ReindexHandler = SearchReindexHandler<SqliteCatalogRepo>;
type ArtworkHandler = ArtworkJobHandler<SqliteCatalogRepo, ImageArtworkPipeline, FsArtworkStore>;
type TrickplayHandler = TrickplayJobHandler<SqliteCatalogRepo, FfmpegTrickplayGenerator>;
type IngestHandler = IngestJobHandler<SqliteLibraryRepo, FfprobeMediaProbe, ServerEnricher>;
type MetadataHandler = MetadataJobHandler<ServerEnricher>;
type RelinkHandler = RelinkJobHandler<SqliteLibraryRepo, FfprobeMediaProbe, ServerEnricher>;
type SubtitlesHandler =
    SubtitlesJobHandler<OpenSubtitlesClient, SqliteCatalogRepo, FsSubtitleStore>;
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
    >,
    FsJobLogStore,
>;
type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub db_root: PathBuf,
    pub bind: SocketAddr,
    pub jwt_secret: String,
    pub stream_secret: String,
    pub access_ttl_secs: i64,
    pub refresh_ttl_secs: i64,
    pub transcode_cache: PathBuf,
    pub artwork_cache: PathBuf,
    pub trickplay_cache: PathBuf,
    pub subtitle_cache: PathBuf,
    pub job_log_dir: PathBuf,
    pub tmdb_api_key: Option<String>,
    pub opensubtitles_api_key: Option<String>,
    pub target_languages: Vec<String>,
    pub worker_concurrency: usize,
    pub worker_period_secs: u64,
    pub scheduler_period_secs: u64,
    pub reaper_period_secs: u64,
    pub shutdown_timeout_secs: u64,
    pub reindex_every_secs: i64,
    pub log_level: String,
    pub sqlx_log_level: String,
    pub bootstrap_mode: BootstrapMode,
    pub bootstrap_dir: PathBuf,
    pub webhook_clients: Vec<::api::WebhookClient>,
    pub basic_client_dir: PathBuf,
    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            db_root: PathBuf::from("data"),
            bind: SocketAddr::from(([0, 0, 0, 0], 8080)),
            jwt_secret: "change-me-jwt-secret".to_owned(),
            stream_secret: "change-me-stream-secret".to_owned(),
            access_ttl_secs: 3600,
            refresh_ttl_secs: 86_400,
            transcode_cache: PathBuf::from("data/transcode"),
            artwork_cache: PathBuf::from("data/artwork"),
            trickplay_cache: PathBuf::from("data/trickplay"),
            subtitle_cache: PathBuf::from("data/subtitles"),
            job_log_dir: PathBuf::from("data/job-logs"),
            tmdb_api_key: None,
            opensubtitles_api_key: None,
            target_languages: vec!["en".to_owned()],
            worker_concurrency: 4,
            worker_period_secs: 5,
            scheduler_period_secs: 30,
            reaper_period_secs: 30,
            shutdown_timeout_secs: 30,
            reindex_every_secs: 3600,
            log_level: "info".to_owned(),
            sqlx_log_level: "warn".to_owned(),
            bootstrap_mode: BootstrapMode::Off,
            bootstrap_dir: PathBuf::from("bootstrap"),
            webhook_clients: Vec::new(),
            basic_client_dir: PathBuf::from("clients/basic"),
            tls_cert: None,
            tls_key: None,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, BoxError> {
        let mut config: Config = Figment::new()
            .merge(Serialized::defaults(Config::default()))
            .merge(Toml::file("shadowmask.toml"))
            .merge(Env::prefixed("SHADOWMASK_").ignore(&["target_languages"]))
            .extract()?;
        config.tmdb_api_key = trim_key(config.tmdb_api_key);
        config.opensubtitles_api_key = trim_key(config.opensubtitles_api_key);
        if let Ok(raw) = std::env::var("SHADOWMASK_TARGET_LANGUAGES") {
            let languages: Vec<String> = raw
                .split(',')
                .map(|language| language.trim().to_owned())
                .filter(|language| !language.is_empty())
                .collect();
            if !languages.is_empty() {
                config.target_languages = languages;
            }
        }
        Ok(config)
    }
}

fn trim_key(key: Option<String>) -> Option<String> {
    key.map(|key| key.trim().to_owned())
        .filter(|key| !key.is_empty())
}

async fn run_bootstrap(config: &Config, repos: &Repos) -> Result<BootstrapResult, BoxError> {
    let library_service = || {
        LibraryServiceImpl::new(
            repos.library.clone(),
            repos.users.clone(),
            repos.jobs.clone(),
            repos.catalog.clone(),
            config.tmdb_api_key.clone().map(TmdbClient::new),
        )
    };
    let providers: Vec<Box<dyn ErasedProvider>> = vec![
        Box::new(LibraryBootstrapProvider::new(library_service())),
        Box::new(UserBootstrapProvider::new(
            UserServiceImpl::new(repos.users.clone()),
            library_service(),
        )),
    ];
    Ok(run_providers(&config.bootstrap_dir, &providers).await)
}

fn box_err<E: std::error::Error + Send + Sync + 'static>(error: E) -> BoxError {
    Box::new(error)
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
        let repos = Repos::connect(&config.db_root).await?;

        let reclaimed = repos.jobs.reclaim_running(Timestamp::now()).await?;
        tracing::info!(reclaimed, "startup: reclaimed stale running jobs");

        let serve = config.bootstrap_mode.serves();
        if config.bootstrap_mode.enabled() {
            run_bootstrap(&config, &repos).await?;
        }

        let wire = WireConfig {
            jwt_secret: config.jwt_secret.into_bytes(),
            stream_secret: config.stream_secret.into_bytes(),
            access_ttl_secs: config.access_ttl_secs,
            refresh_ttl_secs: config.refresh_ttl_secs,
            transcode_cache: config.transcode_cache,
            artwork_cache: config.artwork_cache,
            trickplay_cache: config.trickplay_cache.clone(),
            tmdb_api_key: config.tmdb_api_key.clone(),
        };
        let Built {
            state,
            stream,
            session,
            artwork_store,
            images,
            trickplay,
        } = build_state(&repos, &wire)?;
        let job_logs = FsJobLogStore::new(&config.job_log_dir);
        let router = app(
            state.clone(),
            stream,
            images,
            trickplay,
            config.webhook_clients.clone(),
        )
        .merge(observability_router(metrics, repos.clone()))
        .merge(::api::basic_ui_router(&config.basic_client_dir))
        .merge(::api::job_log_router(
            state,
            ::api::JobLogState::new(job_logs.clone()),
        ));

        let subtitle_langs = if config.opensubtitles_api_key.is_some() {
            config.target_languages.clone()
        } else {
            Vec::new()
        };
        let subtitles = SubtitlesJobHandler::new(
            OpenSubtitlesClient::new(config.opensubtitles_api_key.clone().unwrap_or_default()),
            repos.catalog.clone(),
            FsSubtitleStore::new(&config.subtitle_cache),
        );
        let provider = config.tmdb_api_key.map(TmdbClient::new);
        let scan_enricher = Enricher::new(
            repos.catalog.clone(),
            provider.clone(),
            repos.jobs.clone(),
            repos.library.clone(),
        )
        .with_subtitle_languages(subtitle_langs.clone());
        let ingest_enricher = Enricher::new(
            repos.catalog.clone(),
            provider.clone(),
            repos.jobs.clone(),
            repos.library.clone(),
        )
        .with_subtitle_languages(subtitle_langs.clone());
        let relink_enricher = Enricher::new(
            repos.catalog.clone(),
            provider.clone(),
            repos.jobs.clone(),
            repos.library.clone(),
        )
        .with_subtitle_languages(subtitle_langs);
        let metadata_enricher = Enricher::new(
            repos.catalog.clone(),
            provider,
            repos.jobs.clone(),
            repos.library.clone(),
        );
        let scan = LibraryScanHandler::new(
            repos.library.clone(),
            Scanner::new(WalkdirSourceWalker, FfprobeMediaProbe::default()),
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
        let worker = Worker::new(
            repos.jobs.clone(),
            CompositeJobHandler::new(
                scan, reindex, artwork, trickplay, ingest, metadata, relink, subtitles,
            ),
            job_logs,
            config.worker_concurrency,
            RetryPolicy::default(),
        );

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

        Ok(Self {
            repos,
            router,
            session,
            worker,
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
    use crate::bootstrap::bootstrap_admin;

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

    #[test]
    #[allow(clippy::result_large_err)]
    fn load_uses_defaults_when_unset() {
        figment::Jail::expect_with(|_jail| {
            let config = Config::load().unwrap();
            assert_eq!(config.bind, SocketAddr::from(([0, 0, 0, 0], 8080)));
            assert_eq!(config.worker_concurrency, 4);
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn bootstrap_defaults_to_off() {
        figment::Jail::expect_with(|_jail| {
            let config = Config::load().unwrap();
            assert_eq!(config.bootstrap_mode, BootstrapMode::Off);
            assert_eq!(config.bootstrap_dir, PathBuf::from("bootstrap"));
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn bootstrap_mode_read_from_env() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("SHADOWMASK_BOOTSTRAP_MODE", "init-and-start");
            jail.set_env("SHADOWMASK_BOOTSTRAP_DIR", "/etc/shadowmask/bootstrap");
            let config = Config::load().unwrap();
            assert_eq!(config.bootstrap_mode, BootstrapMode::InitAndStart);
            assert_eq!(
                config.bootstrap_dir,
                PathBuf::from("/etc/shadowmask/bootstrap")
            );
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn load_merges_file_then_env() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "shadowmask.toml",
                "access_ttl_secs = 7\nworker_concurrency = 2\n",
            )?;
            jail.set_env("SHADOWMASK_WORKER_CONCURRENCY", "9");
            let config = Config::load().unwrap();
            assert_eq!(config.access_ttl_secs, 7);
            assert_eq!(config.worker_concurrency, 9);
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn blank_tmdb_api_key_is_treated_as_unset() {
        figment::Jail::expect_with(|jail| {
            assert_eq!(Config::load().unwrap().tmdb_api_key, None);
            jail.set_env("SHADOWMASK_TMDB_API_KEY", "");
            assert_eq!(Config::load().unwrap().tmdb_api_key, None);
            jail.set_env("SHADOWMASK_TMDB_API_KEY", "   ");
            assert_eq!(Config::load().unwrap().tmdb_api_key, None);
            jail.set_env("SHADOWMASK_TMDB_API_KEY", "  real-key  ");
            assert_eq!(
                Config::load().unwrap().tmdb_api_key,
                Some("real-key".to_owned())
            );
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn blank_opensubtitles_api_key_is_treated_as_unset() {
        figment::Jail::expect_with(|jail| {
            assert_eq!(Config::load().unwrap().opensubtitles_api_key, None);
            jail.set_env("SHADOWMASK_OPENSUBTITLES_API_KEY", "   ");
            assert_eq!(Config::load().unwrap().opensubtitles_api_key, None);
            jail.set_env("SHADOWMASK_OPENSUBTITLES_API_KEY", "  os-key  ");
            assert_eq!(
                Config::load().unwrap().opensubtitles_api_key,
                Some("os-key".to_owned())
            );
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn target_languages_default_then_env_comma_split() {
        figment::Jail::expect_with(|jail| {
            assert_eq!(
                Config::load().unwrap().target_languages,
                vec!["en".to_owned()]
            );
            jail.set_env("SHADOWMASK_TARGET_LANGUAGES", "en, es ,, fr");
            assert_eq!(
                Config::load().unwrap().target_languages,
                vec!["en".to_owned(), "es".to_owned(), "fr".to_owned()]
            );
            jail.set_env("SHADOWMASK_TARGET_LANGUAGES", "   ");
            assert_eq!(
                Config::load().unwrap().target_languages,
                vec!["en".to_owned()]
            );
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn webhook_clients_default_empty_and_load_from_toml() {
        figment::Jail::expect_with(|jail| {
            assert!(Config::load().unwrap().webhook_clients.is_empty());
            jail.create_file(
                "shadowmask.toml",
                "[[webhook_clients]]\nname = \"sonarr\"\nsecret = \"abc\"\nlibraries = [\"lib1\"]\n",
            )?;
            let config = Config::load().unwrap();
            assert_eq!(config.webhook_clients.len(), 1);
            assert_eq!(config.webhook_clients[0].name, "sonarr");
            assert_eq!(config.webhook_clients[0].secret, "abc");
            assert_eq!(config.webhook_clients[0].libraries, vec!["lib1".to_owned()]);
            Ok(())
        });
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

    #[test]
    #[allow(clippy::result_large_err)]
    fn shutdown_timeout_read_from_env() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("SHADOWMASK_SHUTDOWN_TIMEOUT_SECS", "7");
            let config = Config::load().unwrap();
            assert_eq!(config.shutdown_timeout_secs, 7);
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn log_level_read_from_env() {
        figment::Jail::expect_with(|jail| {
            let config = Config::load().unwrap();
            assert_eq!(config.log_level, "info");
            assert_eq!(config.sqlx_log_level, "warn");
            jail.set_env("SHADOWMASK_LOG_LEVEL", "debug");
            jail.set_env("SHADOWMASK_SQLX_LOG_LEVEL", "trace");
            let config = Config::load().unwrap();
            assert_eq!(config.log_level, "debug");
            assert_eq!(config.sqlx_log_level, "trace");
            Ok(())
        });
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

        let users = UserServiceImpl::new(repos.users.clone());
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

        let users = UserServiceImpl::new(repos.users.clone());
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
                let users = UserServiceImpl::new(repos.users.clone());
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
        let users = UserServiceImpl::new(runtime.repos.users.clone());
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
        let users = UserServiceImpl::new(runtime.repos.users.clone());
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

    #[test]
    #[allow(clippy::result_large_err)]
    fn tls_paths_default_none_and_load_from_env() {
        figment::Jail::expect_with(|jail| {
            let config = Config::load().unwrap();
            assert!(config.tls_cert.is_none());
            assert!(config.tls_key.is_none());
            jail.set_env("SHADOWMASK_TLS_CERT", "/etc/shadowmask/cert.pem");
            jail.set_env("SHADOWMASK_TLS_KEY", "/etc/shadowmask/key.pem");
            let config = Config::load().unwrap();
            assert_eq!(
                config.tls_cert,
                Some(PathBuf::from("/etc/shadowmask/cert.pem"))
            );
            assert_eq!(
                config.tls_key,
                Some(PathBuf::from("/etc/shadowmask/key.pem"))
            );
            Ok(())
        });
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

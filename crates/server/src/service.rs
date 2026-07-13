use std::future::{Future, IntoFuture, pending};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use axum::Router;
use domain::job::{JobKind, JobPriority};
use figment::Figment;
use figment::providers::{Env, Format, Serialized, Toml};
use jiff::{SignedDuration, Timestamp};
use jobs::{
    ArtworkJobHandler, CompositeJobHandler, IngestJobHandler, JobQueue, LibraryScanHandler,
    MetadataJobHandler, RetryPolicy, Schedule, Scheduler, SearchReindexHandler,
    TrickplayJobHandler, Worker,
};
use media::artwork::FsArtworkStore;
use media::probe::FfprobeMediaProbe;
use media::scan::WalkdirSourceWalker;
use media::trickplay::FfmpegTrickplayGenerator;
use metadata::{ImageArtworkPipeline, TmdbClient};
use persistence::server::{SqliteCatalogRepo, SqliteJobRepo, SqliteLibraryRepo};
use serde::{Deserialize, Serialize};
use services::library::{Enricher, Scanner};
use tokio::net::TcpListener;

use crate::api::{Built, Repos, SessionSvc, WireConfig, app, build_state};

type ServerEnricher = Enricher<SqliteCatalogRepo, TmdbClient, SqliteJobRepo, SqliteLibraryRepo>;
type ScanHandler =
    LibraryScanHandler<SqliteLibraryRepo, WalkdirSourceWalker, FfprobeMediaProbe, ServerEnricher>;
type ReindexHandler = SearchReindexHandler<SqliteCatalogRepo>;
type ArtworkHandler = ArtworkJobHandler<SqliteCatalogRepo, ImageArtworkPipeline, FsArtworkStore>;
type TrickplayHandler = TrickplayJobHandler<SqliteCatalogRepo, FfmpegTrickplayGenerator>;
type IngestHandler = IngestJobHandler<SqliteLibraryRepo, FfprobeMediaProbe, ServerEnricher>;
type MetadataHandler = MetadataJobHandler<ServerEnricher>;
type JobWorker = Worker<
    SqliteJobRepo,
    CompositeJobHandler<
        ScanHandler,
        ReindexHandler,
        ArtworkHandler,
        TrickplayHandler,
        IngestHandler,
        MetadataHandler,
    >,
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
    pub tmdb_api_key: Option<String>,
    pub worker_concurrency: usize,
    pub worker_period_secs: u64,
    pub scheduler_period_secs: u64,
    pub reaper_period_secs: u64,
    pub reindex_every_secs: i64,
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
            tmdb_api_key: None,
            worker_concurrency: 4,
            worker_period_secs: 5,
            scheduler_period_secs: 30,
            reaper_period_secs: 30,
            reindex_every_secs: 3600,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, BoxError> {
        Figment::new()
            .merge(Serialized::defaults(Config::default()))
            .merge(Toml::file("shadowmask.toml"))
            .merge(Env::prefixed("SHADOWMASK_"))
            .extract()
            .map_err(Into::into)
    }
}

pub struct Runtime {
    repos: Repos,
    router: Router,
    session: SessionSvc,
    worker: JobWorker,
    scheduler: Scheduler,
    queue: JobQueue<SqliteJobRepo>,
    bind: SocketAddr,
    worker_period: Duration,
    scheduler_period: Duration,
    reaper_period: Duration,
}

impl Runtime {
    pub async fn build(config: Config) -> Result<Self, BoxError> {
        let repos = Repos::connect(&config.db_root).await?;

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
        let router = app(state, stream, images, trickplay);

        let provider = config.tmdb_api_key.map(TmdbClient::new);
        let scan_enricher = Enricher::new(
            repos.catalog.clone(),
            provider.clone(),
            repos.jobs.clone(),
            repos.library.clone(),
        );
        let ingest_enricher = Enricher::new(
            repos.catalog.clone(),
            provider.clone(),
            repos.jobs.clone(),
            repos.library.clone(),
        );
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
        let worker = Worker::new(
            repos.jobs.clone(),
            CompositeJobHandler::new(scan, reindex, artwork, trickplay, ingest, metadata),
            config.worker_concurrency,
            RetryPolicy::default(),
        );

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
            worker_period: Duration::from_secs(config.worker_period_secs),
            scheduler_period: Duration::from_secs(config.scheduler_period_secs),
            reaper_period: Duration::from_secs(config.reaper_period_secs),
        })
    }

    pub async fn run(self, shutdown: impl Future<Output = ()>) -> Result<(), BoxError> {
        let listener = TcpListener::bind(self.bind).await?;
        let Runtime {
            repos,
            router,
            session,
            worker,
            mut scheduler,
            queue,
            worker_period,
            scheduler_period,
            reaper_period,
            bind: _,
        } = self;

        let reaper = async move {
            let mut ticker = tokio::time::interval(reaper_period);
            loop {
                ticker.tick().await;
                session.reap_idle().await;
            }
        };

        let result: Result<(), BoxError> = tokio::select! {
            biased;
            () = shutdown => Ok(()),
            outcome = worker.run(worker_period, pending::<()>()) => outcome.map_err(Into::into),
            outcome = scheduler.run(scheduler_period, &queue, pending::<()>()) => outcome.map_err(Into::into),
            outcome = axum::serve(listener, router).into_future() => outcome.map_err(Into::into),
            () = reaper => Ok(()),
        };

        repos.close().await;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(db_root: PathBuf) -> Config {
        Config {
            db_root,
            bind: SocketAddr::from(([127, 0, 0, 1], 0)),
            ..Config::default()
        }
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

    #[tokio::test]
    async fn build_wires_a_working_worker() {
        let dir = tempfile::tempdir().unwrap();
        let runtime = Runtime::build(test_config(dir.path().to_owned()))
            .await
            .unwrap();
        assert_eq!(runtime.worker.run_once(Timestamp::now()).await.unwrap(), 0);
        assert_eq!(runtime.session.reap_idle().await, 0);
    }

    #[tokio::test]
    async fn run_drives_background_tasks_until_shutdown() {
        let dir = tempfile::tempdir().unwrap();
        let mut runtime = Runtime::build(test_config(dir.path().to_owned()))
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
}

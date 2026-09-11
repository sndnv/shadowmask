use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Extension;
use axum::Router;
use axum::middleware::from_fn;

use ::api::{
    AppState, Capability, ImageState, ServerCapabilities, StreamState, TrickplayState,
    WebhookClient,
};
use domain::error::{ProfileError, RepositoryError};
use domain::media::CookieInspector;
use fetch::CookieFileInspector;
use jobs::CancelRegistry;
use media::artwork::FsArtworkStore;
use media::hls::HlsStreamSource;
use media::profile::BuiltinProfiles;
use media::stream_token::HmacStreamTokens;
use media::transcode::VideoEncoder;
use metadata::TmdbClient;
use persistence::migrate::migrate_all;
use persistence::server::{
    SqliteAuthTokenRepo, SqliteCatalogRepo, SqliteJobRepo, SqliteLibraryRepo, SqliteUserRepo,
};
use persistence::session::InMemorySessionRegistry;
use persistence::user::{SqlitePreferencesRepo, SqliteProgressRepo, SqliteUserData};
use services::auth::DefaultAuthService;
use services::catalog::CatalogServiceImpl;
use services::discovery::DiscoveryServiceImpl;
use services::job::JobServiceImpl;
use services::library::LibraryServiceImpl;
use services::session::DefaultSessionService;
use services::user::UserServiceImpl;
use services::user_library::UserLibraryServiceImpl;

pub type AuthSvc = DefaultAuthService<SqliteUserRepo, SqliteAuthTokenRepo>;
pub type CatalogSvc = CatalogServiceImpl<SqliteCatalogRepo, SqliteUserRepo>;
pub type SessionSvc = DefaultSessionService<
    SqliteCatalogRepo,
    BuiltinProfiles,
    HlsStreamSource,
    HmacStreamTokens,
    HlsStreamSource,
    InMemorySessionRegistry,
    SqliteUserRepo,
    SqliteProgressRepo,
    SqlitePreferencesRepo,
>;
pub type LibrarySvc = LibraryServiceImpl<
    SqliteLibraryRepo,
    SqliteUserRepo,
    SqliteJobRepo,
    SqliteCatalogRepo,
    TmdbClient,
>;
pub type UserSvc = UserServiceImpl<SqliteUserRepo, SqliteAuthTokenRepo, SqliteUserData>;
pub type UserLibrarySvc =
    UserLibraryServiceImpl<SqliteProgressRepo, SqlitePreferencesRepo, SqliteCatalogRepo>;
pub type DiscoverySvc = DiscoveryServiceImpl<
    SqliteCatalogRepo,
    SqliteCatalogRepo,
    SqliteProgressRepo,
    SqlitePreferencesRepo,
    SqliteUserRepo,
>;
pub type JobSvc = JobServiceImpl<SqliteJobRepo>;

pub type DefaultState = AppState<
    AuthSvc,
    CatalogSvc,
    SessionSvc,
    LibrarySvc,
    UserSvc,
    UserLibrarySvc,
    DiscoverySvc,
    JobSvc,
>;
pub type DefaultStreamState = StreamState<HmacStreamTokens, HlsStreamSource>;

pub struct WireConfig {
    pub jwt_secret: Vec<u8>,
    pub stream_secret: Vec<u8>,
    pub access_ttl_secs: i64,
    pub refresh_ttl_secs: i64,
    pub transcode_cache: PathBuf,
    pub artwork_cache: PathBuf,
    pub trickplay_cache: PathBuf,
    pub tmdb_api_key: Option<String>,
    pub transcription_enabled: bool,
    pub translation_enabled: bool,
    pub upscaling_enabled: bool,
    pub content_fetch_enabled: bool,
    pub fetch_cookies_file: Option<PathBuf>,
    pub vaapi_device: Option<String>,
    pub remux_read_rate: f64,
    pub max_transcode_height: Option<u32>,
    pub profile_overrides_dir: Option<PathBuf>,
}

#[derive(Clone)]
pub struct Repos {
    pub catalog: SqliteCatalogRepo,
    pub library: SqliteLibraryRepo,
    pub users: SqliteUserRepo,
    pub jobs: SqliteJobRepo,
    pub auth_tokens: SqliteAuthTokenRepo,
    pub progress: SqliteProgressRepo,
    pub preferences: SqlitePreferencesRepo,
    pub user_data: SqliteUserData,
}

impl Repos {
    pub async fn connect(db_root: &Path) -> Result<Self, RepositoryError> {
        migrate_all(db_root).await?;
        let server = db_root.join("server");
        let users_dir = db_root.join("users");
        let progress = SqliteProgressRepo::new(&users_dir);
        let preferences = SqlitePreferencesRepo::new(&users_dir);
        Ok(Self {
            catalog: SqliteCatalogRepo::connect(&server.join("catalog.db")).await?,
            library: SqliteLibraryRepo::connect(&server.join("libraries.db")).await?,
            users: SqliteUserRepo::connect(&server.join("users.db")).await?,
            jobs: SqliteJobRepo::connect(&server.join("jobs.db")).await?,
            auth_tokens: SqliteAuthTokenRepo::connect(&server.join("auth.db")).await?,
            user_data: SqliteUserData::new(progress.clone(), preferences.clone()),
            progress,
            preferences,
        })
    }

    pub async fn ready(&self) -> bool {
        self.catalog.ping().await.is_ok()
            && self.library.ping().await.is_ok()
            && self.users.ping().await.is_ok()
            && self.jobs.ping().await.is_ok()
            && self.auth_tokens.ping().await.is_ok()
    }

    pub async fn close(&self) {
        self.catalog.close().await;
        self.library.close().await;
        self.users.close().await;
        self.jobs.close().await;
        self.auth_tokens.close().await;
        self.progress.close().await;
        self.preferences.close().await;
    }
}

pub struct Built {
    pub state: DefaultState,
    pub stream: DefaultStreamState,
    pub session: SessionSvc,
    pub artwork_store: FsArtworkStore,
    pub images: ImageState,
    pub trickplay: TrickplayState,
}

pub fn build_state(
    repos: &Repos,
    cfg: &WireConfig,
    cancel: &CancelRegistry,
) -> Result<Built, ProfileError> {
    let profiles = BuiltinProfiles::load_from_dir(cfg.profile_overrides_dir.as_deref())?;
    let hls = HlsStreamSource::new(&cfg.transcode_cache)
        .with_encoder(VideoEncoder::from_device(cfg.vaapi_device.clone()))
        .with_read_rate(cfg.remux_read_rate);
    let artwork_store = FsArtworkStore::new(&cfg.artwork_cache);
    let images = ImageState::new(&cfg.artwork_cache);
    let trickplay = TrickplayState::new(&cfg.trickplay_cache);

    let auth = DefaultAuthService::new(
        repos.users.clone(),
        repos.auth_tokens.clone(),
        &cfg.jwt_secret,
        cfg.access_ttl_secs,
        cfg.refresh_ttl_secs,
    );
    let catalog = CatalogServiceImpl::new(repos.catalog.clone(), repos.users.clone());
    let session = DefaultSessionService::new(
        repos.catalog.clone(),
        profiles,
        hls.clone(),
        HmacStreamTokens::new(&cfg.stream_secret),
        hls.clone(),
        InMemorySessionRegistry::new(),
        repos.users.clone(),
        repos.progress.clone(),
        repos.preferences.clone(),
    )
    .with_max_transcode_height(cfg.max_transcode_height);
    let provider = cfg.tmdb_api_key.clone().map(TmdbClient::new);
    let library = LibraryServiceImpl::new(
        repos.library.clone(),
        repos.users.clone(),
        repos.jobs.clone(),
        repos.catalog.clone(),
        provider,
    )
    .with_enrichment_flags(
        cfg.transcription_enabled,
        cfg.translation_enabled,
        cfg.upscaling_enabled,
    )
    .with_content_fetch(cfg.content_fetch_enabled)
    .with_cookie_inspector(
        cfg.fetch_cookies_file
            .clone()
            .map(|path| Arc::new(CookieFileInspector::new(path)) as Arc<dyn CookieInspector>),
    );
    let job = JobServiceImpl::new(repos.jobs.clone()).with_canceller(Arc::new(cancel.clone()));
    let user = UserServiceImpl::new(
        repos.users.clone(),
        repos.auth_tokens.clone(),
        repos.user_data.clone(),
    );
    let user_library = UserLibraryServiceImpl::new(
        Arc::new(repos.progress.clone()),
        Arc::new(repos.preferences.clone()),
        Arc::new(repos.catalog.clone()),
    );
    let discovery = DiscoveryServiceImpl::new(
        repos.catalog.clone(),
        repos.catalog.clone(),
        repos.progress.clone(),
        repos.preferences.clone(),
        repos.users.clone(),
    );

    let state =
        AppState::new(auth, catalog, session.clone(), library, user, user_library, discovery, job);
    let stream = StreamState::new(HmacStreamTokens::new(&cfg.stream_secret), hls);
    Ok(Built { state, stream, session, artwork_store, images, trickplay })
}

pub fn app(
    state: DefaultState,
    stream: DefaultStreamState,
    images: ImageState,
    trickplay: TrickplayState,
    webhook_clients: Vec<WebhookClient>,
    capabilities: Vec<Capability>,
) -> Router {
    ::api::router(state.clone())
        .merge(::api::stream_router(stream))
        .merge(::api::image_router(images))
        .merge(::api::webhook_router(state.clone(), webhook_clients))
        .merge(::api::trickplay_router(state, trickplay))
        .layer(from_fn(::api::middleware::track_http))
        .layer(Extension(ServerCapabilities(capabilities)))
}

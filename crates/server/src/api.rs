use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Router;

use ::api::{AppState, ImageState, StreamState, TrickplayState};
use domain::error::{ProfileError, RepositoryError};
use media::artwork::FsArtworkStore;
use media::hls::HlsStreamSource;
use media::profile::BuiltinProfiles;
use media::stream_token::HmacStreamTokens;
use media::transcode::FfmpegTranscodeManager;
use metadata::TmdbClient;
use persistence::migrate::migrate_all;
use persistence::server::{
    SqliteAuthTokenRepo, SqliteCatalogRepo, SqliteJobRepo, SqliteLibraryRepo, SqliteUserRepo,
};
use persistence::session::InMemorySessionRegistry;
use persistence::user::{SqlitePreferencesRepo, SqliteProgressRepo};
use services::auth::DefaultAuthService;
use services::catalog::CatalogServiceImpl;
use services::discovery::DiscoveryServiceImpl;
use services::library::LibraryServiceImpl;
use services::session::DefaultSessionService;
use services::user::UserServiceImpl;
use services::user_library::UserLibraryServiceImpl;

pub type AuthSvc = DefaultAuthService<SqliteUserRepo, SqliteAuthTokenRepo>;
pub type CatalogSvc = CatalogServiceImpl<SqliteCatalogRepo, SqliteUserRepo>;
pub type SessionSvc = DefaultSessionService<
    SqliteCatalogRepo,
    BuiltinProfiles,
    FfmpegTranscodeManager,
    HmacStreamTokens,
    HlsStreamSource,
    InMemorySessionRegistry,
    SqliteUserRepo,
    SqliteProgressRepo,
>;
pub type LibrarySvc = LibraryServiceImpl<
    SqliteLibraryRepo,
    SqliteUserRepo,
    SqliteJobRepo,
    SqliteCatalogRepo,
    TmdbClient,
>;
pub type UserSvc = UserServiceImpl<SqliteUserRepo>;
pub type UserLibrarySvc =
    UserLibraryServiceImpl<SqliteProgressRepo, SqlitePreferencesRepo, SqliteCatalogRepo>;
pub type DiscoverySvc =
    DiscoveryServiceImpl<SqliteCatalogRepo, SqliteCatalogRepo, SqliteProgressRepo>;

pub type DefaultState =
    AppState<AuthSvc, CatalogSvc, SessionSvc, LibrarySvc, UserSvc, UserLibrarySvc, DiscoverySvc>;
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
}

pub struct Repos {
    pub catalog: SqliteCatalogRepo,
    pub library: SqliteLibraryRepo,
    pub users: SqliteUserRepo,
    pub jobs: SqliteJobRepo,
    pub auth_tokens: SqliteAuthTokenRepo,
    pub progress: SqliteProgressRepo,
    pub preferences: SqlitePreferencesRepo,
}

impl Repos {
    pub async fn connect(db_root: &Path) -> Result<Self, RepositoryError> {
        migrate_all(db_root).await?;
        let server = db_root.join("server");
        let users_dir = db_root.join("users");
        Ok(Self {
            catalog: SqliteCatalogRepo::connect(&server.join("catalog.db")).await?,
            library: SqliteLibraryRepo::connect(&server.join("libraries.db")).await?,
            users: SqliteUserRepo::connect(&server.join("users.db")).await?,
            jobs: SqliteJobRepo::connect(&server.join("jobs.db")).await?,
            auth_tokens: SqliteAuthTokenRepo::connect(&server.join("auth.db")).await?,
            progress: SqliteProgressRepo::new(&users_dir),
            preferences: SqlitePreferencesRepo::new(&users_dir),
        })
    }

    pub async fn close(&self) {
        self.catalog.close().await;
        self.library.close().await;
        self.users.close().await;
        self.jobs.close().await;
        self.auth_tokens.close().await;
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

pub fn build_state(repos: &Repos, cfg: &WireConfig) -> Result<Built, ProfileError> {
    let profiles = BuiltinProfiles::load()?;
    let transcode = FfmpegTranscodeManager::new(&cfg.transcode_cache);
    let hls = HlsStreamSource::new();
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
        transcode,
        HmacStreamTokens::new(&cfg.stream_secret),
        hls.clone(),
        InMemorySessionRegistry::new(),
        repos.users.clone(),
        repos.progress.clone(),
    );
    let provider = cfg.tmdb_api_key.clone().map(TmdbClient::new);
    let library = LibraryServiceImpl::new(
        repos.library.clone(),
        repos.users.clone(),
        repos.jobs.clone(),
        repos.catalog.clone(),
        provider,
    );
    let user = UserServiceImpl::new(repos.users.clone());
    let user_library = UserLibraryServiceImpl::new(
        Arc::new(repos.progress.clone()),
        Arc::new(repos.preferences.clone()),
        Arc::new(repos.catalog.clone()),
    );
    let discovery = DiscoveryServiceImpl::new(
        repos.catalog.clone(),
        repos.catalog.clone(),
        repos.progress.clone(),
    );

    let state = AppState::new(
        auth,
        catalog,
        session.clone(),
        library,
        user,
        user_library,
        discovery,
    );
    let stream = StreamState::new(HmacStreamTokens::new(&cfg.stream_secret), hls);
    Ok(Built {
        state,
        stream,
        session,
        artwork_store,
        images,
        trickplay,
    })
}

pub fn app(
    state: DefaultState,
    stream: DefaultStreamState,
    images: ImageState,
    trickplay: TrickplayState,
) -> Router {
    ::api::router(state.clone())
        .merge(::api::stream_router(stream))
        .merge(::api::image_router(images))
        .merge(::api::trickplay_router(state, trickplay))
}

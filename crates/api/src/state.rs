use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use domain::service::{
    AuthService, CatalogService, DiscoveryService, JobService, LibraryService, SessionService,
    UserLibraryService, UserService,
};

#[derive(Clone)]
pub struct AppState<A, C, Se, L, U, Ul, D, Jb> {
    pub auth: A,
    pub catalog: C,
    pub session: Se,
    pub library: L,
    pub user: U,
    pub user_library: Ul,
    pub discovery: D,
    pub job: Jb,
}

impl<A, C, Se, L, U, Ul, D, Jb> AppState<A, C, Se, L, U, Ul, D, Jb> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        auth: A,
        catalog: C,
        session: Se,
        library: L,
        user: U,
        user_library: Ul,
        discovery: D,
        job: Jb,
    ) -> Self {
        Self { auth, catalog, session, library, user, user_library, discovery, job }
    }
}

pub struct StreamState<T, G> {
    pub tokens: Arc<T>,
    pub source: Arc<G>,
}

impl<T, G> StreamState<T, G> {
    pub fn new(tokens: T, source: G) -> Self {
        Self { tokens: Arc::new(tokens), source: Arc::new(source) }
    }
}

impl<T, G> Clone for StreamState<T, G> {
    fn clone(&self) -> Self {
        Self { tokens: Arc::clone(&self.tokens), source: Arc::clone(&self.source) }
    }
}

#[derive(Clone)]
pub struct ImageState {
    pub root: Arc<PathBuf>,
}

impl ImageState {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: Arc::new(root.into()) }
    }
}

#[derive(Clone)]
pub struct TrickplayState {
    pub root: Arc<PathBuf>,
}

impl TrickplayState {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: Arc::new(root.into()) }
    }
}

pub struct JobLogState<J> {
    pub store: Arc<J>,
}

impl<J> JobLogState<J> {
    pub fn new(store: J) -> Self {
        Self { store: Arc::new(store) }
    }
}

impl<J> Clone for JobLogState<J> {
    fn clone(&self) -> Self {
        Self { store: Arc::clone(&self.store) }
    }
}

pub struct DownloadState<S, C, D> {
    pub services: S,
    pub catalog: Arc<C>,
    pub tokens: Arc<D>,
}

impl<S, C, D> DownloadState<S, C, D> {
    pub fn new(services: S, catalog: C, tokens: D) -> Self {
        Self { services, catalog: Arc::new(catalog), tokens: Arc::new(tokens) }
    }
}

impl<S: Clone, C, D> Clone for DownloadState<S, C, D> {
    fn clone(&self) -> Self {
        Self {
            services: self.services.clone(),
            catalog: Arc::clone(&self.catalog),
            tokens: Arc::clone(&self.tokens),
        }
    }
}

pub struct SubtitleState<C, S> {
    pub catalog: Arc<C>,
    pub subtitles: Arc<S>,
}

impl<C, S> SubtitleState<C, S> {
    pub fn new(catalog: C, subtitles: S) -> Self {
        Self { catalog: Arc::new(catalog), subtitles: Arc::new(subtitles) }
    }
}

impl<C, S> Clone for SubtitleState<C, S> {
    fn clone(&self) -> Self {
        Self { catalog: Arc::clone(&self.catalog), subtitles: Arc::clone(&self.subtitles) }
    }
}

pub struct SubtitleSearchState<C, S, P> {
    pub catalog: Arc<C>,
    pub subtitles: Arc<S>,
    pub provider: Arc<P>,
}

impl<C, S, P> SubtitleSearchState<C, S, P> {
    pub fn new(catalog: C, subtitles: S, provider: P) -> Self {
        Self {
            catalog: Arc::new(catalog),
            subtitles: Arc::new(subtitles),
            provider: Arc::new(provider),
        }
    }
}

impl<C, S, P> Clone for SubtitleSearchState<C, S, P> {
    fn clone(&self) -> Self {
        Self {
            catalog: Arc::clone(&self.catalog),
            subtitles: Arc::clone(&self.subtitles),
            provider: Arc::clone(&self.provider),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookClient {
    pub name: String,
    pub secret: String,
    #[serde(default)]
    pub libraries: Vec<String>,
}

#[derive(Clone)]
pub struct WebhookState<S> {
    pub services: S,
    pub clients: Vec<WebhookClient>,
}

#[derive(Debug, Clone, Default)]
pub struct ServerCapabilities(pub Vec<crate::dto::server::Capability>);

pub trait AppServices: Clone + Send + Sync + 'static {
    type Auth: AuthService + Send + Sync;
    type Catalog: CatalogService + Send + Sync;
    type Session: SessionService + Send + Sync;
    type Library: LibraryService + Send + Sync;
    type User: UserService + Send + Sync;
    type UserLibrary: UserLibraryService + Send + Sync;
    type Discovery: DiscoveryService + Send + Sync;
    type Job: JobService + Send + Sync;

    fn auth(&self) -> &Self::Auth;
    fn catalog(&self) -> &Self::Catalog;
    fn session(&self) -> &Self::Session;
    fn library(&self) -> &Self::Library;
    fn user(&self) -> &Self::User;
    fn user_library(&self) -> &Self::UserLibrary;
    fn discovery(&self) -> &Self::Discovery;
    fn job(&self) -> &Self::Job;
}

impl<A, C, Se, L, U, Ul, D, Jb> AppServices for AppState<A, C, Se, L, U, Ul, D, Jb>
where
    A: AuthService + Clone + Send + Sync + 'static,
    C: CatalogService + Clone + Send + Sync + 'static,
    Se: SessionService + Clone + Send + Sync + 'static,
    L: LibraryService + Clone + Send + Sync + 'static,
    U: UserService + Clone + Send + Sync + 'static,
    Ul: UserLibraryService + Clone + Send + Sync + 'static,
    D: DiscoveryService + Clone + Send + Sync + 'static,
    Jb: JobService + Clone + Send + Sync + 'static,
{
    type Auth = A;
    type Catalog = C;
    type Session = Se;
    type Library = L;
    type User = U;
    type UserLibrary = Ul;
    type Discovery = D;
    type Job = Jb;

    fn auth(&self) -> &A {
        &self.auth
    }

    fn catalog(&self) -> &C {
        &self.catalog
    }

    fn session(&self) -> &Se {
        &self.session
    }

    fn library(&self) -> &L {
        &self.library
    }

    fn user(&self) -> &U {
        &self.user
    }

    fn user_library(&self) -> &Ul {
        &self.user_library
    }

    fn discovery(&self) -> &D {
        &self.discovery
    }

    fn job(&self) -> &Jb {
        &self.job
    }
}

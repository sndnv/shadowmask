use std::path::PathBuf;
use std::sync::Arc;

use domain::catalog::{
    Collection, CollectionId, CollectionUpdate, Episode, EpisodeId, Movie, MovieDetail, MovieId,
    NewCollection, PersonProfile, Season, SeasonId, Series, SeriesDetail, SeriesId, TitleCard,
    TitleId, TitleListQuery, TitleRef, Version, VersionDetail, VersionId,
};
use domain::common::{Page, PageRequest};
use domain::discovery::{ContinueWatchingItem, Hub, SearchKind, SearchResult};
use domain::error::{
    AuthError, CatalogError, DiscoveryError, LibraryError, SessionError, UserError,
};
use domain::job::Job;
use domain::library::{
    DuplicateCandidate, DuplicateCandidateId, Library, LibraryId, LibraryUpdate, NewLibrary,
    ResolveCandidate, ResolveTarget, ScanState, UnmatchedFile, UnmatchedFileId,
};
use domain::metadata::{ExternalId, Genre, PersonId};
use domain::playback::{
    Favorite, PlaybackProgress, TitleState, WatchHistory, WatchTarget, WatchlistItem,
};
use domain::service::{
    AuthService, CatalogService, DiscoveryService, LibraryService, SessionService,
    UserLibraryService, UserService,
};
use domain::session::{
    HeartbeatAck, NowPlaying, PlaybackSession, PlaybackState, Renegotiated, SessionId,
    SessionStarted, SessionUpdate, StartSessionRequest,
};
use domain::user::{
    ApiToken, ApiTokenId, Device, DeviceId, DeviceRegistration, IssuedToken, LibraryAccess,
    NewUser, PendingLink, Principal, TokenPair, User, UserId, UserProfileUpdate,
};

#[derive(Clone)]
pub struct AppState<A, C, Se, L, U, Ul, D> {
    pub auth: A,
    pub catalog: C,
    pub session: Se,
    pub library: L,
    pub user: U,
    pub user_library: Ul,
    pub discovery: D,
}

impl<A, C, Se, L, U, Ul, D> AppState<A, C, Se, L, U, Ul, D> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        auth: A,
        catalog: C,
        session: Se,
        library: L,
        user: U,
        user_library: Ul,
        discovery: D,
    ) -> Self {
        Self {
            auth,
            catalog,
            session,
            library,
            user,
            user_library,
            discovery,
        }
    }
}

pub struct StreamState<T, G> {
    pub tokens: Arc<T>,
    pub source: Arc<G>,
}

impl<T, G> StreamState<T, G> {
    pub fn new(tokens: T, source: G) -> Self {
        Self {
            tokens: Arc::new(tokens),
            source: Arc::new(source),
        }
    }
}

impl<T, G> Clone for StreamState<T, G> {
    fn clone(&self) -> Self {
        Self {
            tokens: Arc::clone(&self.tokens),
            source: Arc::clone(&self.source),
        }
    }
}

#[derive(Clone)]
pub struct ImageState {
    pub root: Arc<PathBuf>,
}

impl ImageState {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: Arc::new(root.into()),
        }
    }
}

#[derive(Clone)]
pub struct TrickplayState {
    pub root: Arc<PathBuf>,
}

impl TrickplayState {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: Arc::new(root.into()),
        }
    }
}

impl<A, C, Se, L, U, Ul, D> AuthService for AppState<A, C, Se, L, U, Ul, D>
where
    A: AuthService + Sync,
    C: Sync,
    Se: Sync,
    L: Sync,
    U: Sync,
    Ul: Sync,
    D: Sync,
{
    async fn login(&self, username: &str, password: &str) -> Result<TokenPair, AuthError> {
        self.auth.login(username, password).await
    }

    async fn refresh(&self, refresh_token: &str) -> Result<TokenPair, AuthError> {
        self.auth.refresh(refresh_token).await
    }

    async fn redeem_link_code(
        &self,
        code: &str,
        device: DeviceRegistration,
    ) -> Result<IssuedToken, AuthError> {
        self.auth.redeem_link_code(code, device).await
    }

    async fn authenticate(&self, access_token: &str) -> Result<Principal, AuthError> {
        self.auth.authenticate(access_token).await
    }

    async fn create_link_code(
        &self,
        caller: &Principal,
        user: Option<UserId>,
        ttl_secs: Option<i64>,
    ) -> Result<PendingLink, AuthError> {
        self.auth.create_link_code(caller, user, ttl_secs).await
    }

    async fn logout(&self, refresh_token: &str) -> Result<(), AuthError> {
        self.auth.logout(refresh_token).await
    }

    async fn logout_all(&self, user: &UserId) -> Result<(), AuthError> {
        self.auth.logout_all(user).await
    }

    async fn list_devices(&self, user: &UserId) -> Result<Vec<Device>, AuthError> {
        self.auth.list_devices(user).await
    }

    async fn list_api_tokens(&self, user: &UserId) -> Result<Vec<ApiToken>, AuthError> {
        self.auth.list_api_tokens(user).await
    }

    async fn revoke_device(&self, user: &UserId, device: &DeviceId) -> Result<(), AuthError> {
        self.auth.revoke_device(user, device).await
    }

    async fn revoke_api_token(&self, user: &UserId, token: &ApiTokenId) -> Result<(), AuthError> {
        self.auth.revoke_api_token(user, token).await
    }
}

impl<A, C, Se, L, U, Ul, D> CatalogService for AppState<A, C, Se, L, U, Ul, D>
where
    A: Sync,
    C: CatalogService + Sync,
    Se: Sync,
    L: Sync,
    U: Sync,
    Ul: Sync,
    D: Sync,
{
    async fn collections(
        &self,
        caller: &Principal,
        page: PageRequest,
    ) -> Result<Page<Collection>, CatalogError> {
        self.catalog.collections(caller, page).await
    }

    async fn collection(
        &self,
        caller: &Principal,
        id: &CollectionId,
    ) -> Result<Collection, CatalogError> {
        self.catalog.collection(caller, id).await
    }

    async fn create_collection(
        &self,
        caller: &Principal,
        input: NewCollection,
    ) -> Result<Collection, CatalogError> {
        self.catalog.create_collection(caller, input).await
    }

    async fn update_collection(
        &self,
        caller: &Principal,
        id: &CollectionId,
        update: CollectionUpdate,
    ) -> Result<Collection, CatalogError> {
        self.catalog.update_collection(caller, id, update).await
    }

    async fn delete_collection(
        &self,
        caller: &Principal,
        id: &CollectionId,
    ) -> Result<(), CatalogError> {
        self.catalog.delete_collection(caller, id).await
    }

    async fn movies(
        &self,
        caller: &Principal,
        query: &TitleListQuery,
        page: PageRequest,
    ) -> Result<Page<Movie>, CatalogError> {
        self.catalog.movies(caller, query, page).await
    }

    async fn movie(&self, caller: &Principal, id: &MovieId) -> Result<MovieDetail, CatalogError> {
        self.catalog.movie(caller, id).await
    }

    async fn series(
        &self,
        caller: &Principal,
        query: &TitleListQuery,
        page: PageRequest,
    ) -> Result<Page<Series>, CatalogError> {
        self.catalog.series(caller, query, page).await
    }

    async fn series_detail(
        &self,
        caller: &Principal,
        id: &SeriesId,
    ) -> Result<SeriesDetail, CatalogError> {
        self.catalog.series_detail(caller, id).await
    }

    async fn seasons(
        &self,
        caller: &Principal,
        series: &SeriesId,
    ) -> Result<Vec<Season>, CatalogError> {
        self.catalog.seasons(caller, series).await
    }

    async fn season(&self, caller: &Principal, id: &SeasonId) -> Result<Season, CatalogError> {
        self.catalog.season(caller, id).await
    }

    async fn episodes(
        &self,
        caller: &Principal,
        season: &SeasonId,
    ) -> Result<Vec<Episode>, CatalogError> {
        self.catalog.episodes(caller, season).await
    }

    async fn episode(&self, caller: &Principal, id: &EpisodeId) -> Result<Episode, CatalogError> {
        self.catalog.episode(caller, id).await
    }

    async fn versions(
        &self,
        caller: &Principal,
        title: &TitleId,
        page: PageRequest,
    ) -> Result<Page<Version>, CatalogError> {
        self.catalog.versions(caller, title, page).await
    }

    async fn library_versions(
        &self,
        caller: &Principal,
        library: &LibraryId,
        page: PageRequest,
    ) -> Result<Page<Version>, CatalogError> {
        self.catalog.library_versions(caller, library, page).await
    }

    async fn version(
        &self,
        caller: &Principal,
        id: &VersionId,
    ) -> Result<VersionDetail, CatalogError> {
        self.catalog.version(caller, id).await
    }

    async fn person(
        &self,
        caller: &Principal,
        id: &PersonId,
    ) -> Result<PersonProfile, CatalogError> {
        self.catalog.person(caller, id).await
    }

    async fn genres(&self, caller: &Principal) -> Result<Vec<Genre>, CatalogError> {
        self.catalog.genres(caller).await
    }

    async fn title_cards(
        &self,
        caller: &Principal,
        ids: &[TitleId],
    ) -> Result<Vec<TitleCard>, CatalogError> {
        self.catalog.title_cards(caller, ids).await
    }
}

impl<A, C, Se, L, U, Ul, D> SessionService for AppState<A, C, Se, L, U, Ul, D>
where
    A: Sync,
    C: Sync,
    Se: SessionService + Sync,
    L: Sync,
    U: Sync,
    Ul: Sync,
    D: Sync,
{
    async fn start(
        &self,
        caller: &Principal,
        request: StartSessionRequest,
    ) -> Result<SessionStarted, SessionError> {
        self.session.start(caller, request).await
    }

    async fn heartbeat(
        &self,
        caller: &Principal,
        session: &SessionId,
        position_ms: u64,
        state: PlaybackState,
    ) -> Result<HeartbeatAck, SessionError> {
        self.session
            .heartbeat(caller, session, position_ms, state)
            .await
    }

    async fn seek(
        &self,
        caller: &Principal,
        session: &SessionId,
        position_ms: u64,
    ) -> Result<Renegotiated, SessionError> {
        self.session.seek(caller, session, position_ms).await
    }

    async fn update(
        &self,
        caller: &Principal,
        session: &SessionId,
        update: SessionUpdate,
    ) -> Result<Renegotiated, SessionError> {
        self.session.update(caller, session, update).await
    }

    async fn end(&self, caller: &Principal, session: &SessionId) -> Result<(), SessionError> {
        self.session.end(caller, session).await
    }

    async fn active_sessions(
        &self,
        caller: &Principal,
        page: PageRequest,
    ) -> Result<Page<PlaybackSession>, SessionError> {
        self.session.active_sessions(caller, page).await
    }
}

impl<A, C, Se, L, U, Ul, D> LibraryService for AppState<A, C, Se, L, U, Ul, D>
where
    A: Sync,
    C: Sync,
    Se: Sync,
    L: LibraryService + Sync,
    U: Sync,
    Ul: Sync,
    D: Sync,
{
    async fn libraries(&self, caller: &Principal) -> Result<Vec<Library>, LibraryError> {
        self.library.libraries(caller).await
    }

    async fn library(&self, caller: &Principal, id: &LibraryId) -> Result<Library, LibraryError> {
        self.library.library(caller, id).await
    }

    async fn create_library(
        &self,
        caller: &Principal,
        input: NewLibrary,
    ) -> Result<Library, LibraryError> {
        self.library.create_library(caller, input).await
    }

    async fn update_library(
        &self,
        caller: &Principal,
        id: &LibraryId,
        update: LibraryUpdate,
    ) -> Result<Library, LibraryError> {
        self.library.update_library(caller, id, update).await
    }

    async fn delete_library(&self, caller: &Principal, id: &LibraryId) -> Result<(), LibraryError> {
        self.library.delete_library(caller, id).await
    }

    async fn scan_state(
        &self,
        caller: &Principal,
        id: &LibraryId,
    ) -> Result<ScanState, LibraryError> {
        self.library.scan_state(caller, id).await
    }

    async fn trigger_scan(&self, caller: &Principal, id: &LibraryId) -> Result<(), LibraryError> {
        self.library.trigger_scan(caller, id).await
    }

    async fn unmatched(
        &self,
        caller: &Principal,
        id: &LibraryId,
        page: PageRequest,
    ) -> Result<Page<UnmatchedFile>, LibraryError> {
        self.library.unmatched(caller, id, page).await
    }

    async fn duplicates(
        &self,
        caller: &Principal,
        id: &LibraryId,
        page: PageRequest,
    ) -> Result<Page<DuplicateCandidate>, LibraryError> {
        self.library.duplicates(caller, id, page).await
    }

    async fn unmatched_candidates(
        &self,
        caller: &Principal,
        id: &LibraryId,
        unmatched: &UnmatchedFileId,
        query: Option<String>,
    ) -> Result<Vec<ResolveCandidate>, LibraryError> {
        self.library
            .unmatched_candidates(caller, id, unmatched, query)
            .await
    }

    async fn resolve_unmatched(
        &self,
        caller: &Principal,
        id: &LibraryId,
        unmatched: &UnmatchedFileId,
        target: ResolveTarget,
    ) -> Result<(), LibraryError> {
        self.library
            .resolve_unmatched(caller, id, unmatched, target)
            .await
    }

    async fn dismiss_duplicate(
        &self,
        caller: &Principal,
        id: &LibraryId,
        duplicate: &DuplicateCandidateId,
    ) -> Result<(), LibraryError> {
        self.library.dismiss_duplicate(caller, id, duplicate).await
    }

    async fn resolve_duplicate(
        &self,
        caller: &Principal,
        id: &LibraryId,
        duplicate: &DuplicateCandidateId,
    ) -> Result<(), LibraryError> {
        self.library.resolve_duplicate(caller, id, duplicate).await
    }

    async fn reidentify(
        &self,
        caller: &Principal,
        title: TitleRef,
        external_id: Option<ExternalId>,
    ) -> Result<(), LibraryError> {
        self.library.reidentify(caller, title, external_id).await
    }

    async fn jobs(&self, caller: &Principal) -> Result<Vec<Job>, LibraryError> {
        self.library.jobs(caller).await
    }
}

impl<A, C, Se, L, U, Ul, D> UserService for AppState<A, C, Se, L, U, Ul, D>
where
    A: Sync,
    C: Sync,
    Se: Sync,
    L: Sync,
    U: UserService + Sync,
    Ul: Sync,
    D: Sync,
{
    async fn create(&self, input: NewUser) -> Result<User, UserError> {
        self.user.create(input).await
    }

    async fn get(&self, id: &UserId) -> Result<User, UserError> {
        self.user.get(id).await
    }

    async fn list(&self, page: PageRequest) -> Result<Page<User>, UserError> {
        self.user.list(page).await
    }

    async fn update_profile(
        &self,
        id: &UserId,
        update: UserProfileUpdate,
    ) -> Result<User, UserError> {
        self.user.update_profile(id, update).await
    }

    async fn delete(&self, id: &UserId) -> Result<(), UserError> {
        self.user.delete(id).await
    }

    async fn library_access(&self, id: &UserId) -> Result<Vec<LibraryAccess>, UserError> {
        self.user.library_access(id).await
    }

    async fn set_library_access(
        &self,
        id: &UserId,
        libraries: &[LibraryId],
    ) -> Result<(), UserError> {
        self.user.set_library_access(id, libraries).await
    }

    async fn change_password(
        &self,
        actor: &Principal,
        target: &UserId,
        current: Option<&str>,
        new_password: &str,
    ) -> Result<(), UserError> {
        self.user
            .change_password(actor, target, current, new_password)
            .await
    }
}

impl<A, C, Se, L, U, Ul, D> UserLibraryService for AppState<A, C, Se, L, U, Ul, D>
where
    A: Sync,
    C: Sync,
    Se: Sync,
    L: Sync,
    U: Sync,
    Ul: UserLibraryService + Sync,
    D: Sync,
{
    async fn watchlist(&self, user: &UserId) -> Result<Vec<WatchlistItem>, UserError> {
        self.user_library.watchlist(user).await
    }

    async fn add_to_watchlist(&self, user: &UserId, title: &TitleId) -> Result<(), UserError> {
        self.user_library.add_to_watchlist(user, title).await
    }

    async fn remove_from_watchlist(&self, user: &UserId, title_id: &str) -> Result<(), UserError> {
        self.user_library
            .remove_from_watchlist(user, title_id)
            .await
    }

    async fn favorites(&self, user: &UserId) -> Result<Vec<Favorite>, UserError> {
        self.user_library.favorites(user).await
    }

    async fn add_favorite(&self, user: &UserId, title: &TitleId) -> Result<(), UserError> {
        self.user_library.add_favorite(user, title).await
    }

    async fn remove_favorite(&self, user: &UserId, title_id: &str) -> Result<(), UserError> {
        self.user_library.remove_favorite(user, title_id).await
    }

    async fn history(
        &self,
        user: &UserId,
        page: PageRequest,
    ) -> Result<Page<WatchHistory>, UserError> {
        self.user_library.history(user, page).await
    }

    async fn progress(
        &self,
        user: &UserId,
        version: &VersionId,
    ) -> Result<Option<PlaybackProgress>, UserError> {
        self.user_library.progress(user, version).await
    }

    async fn clear_progress(&self, user: &UserId, version: &VersionId) -> Result<(), UserError> {
        self.user_library.clear_progress(user, version).await
    }

    async fn set_watched(
        &self,
        user: &UserId,
        target: &WatchTarget,
        watched: bool,
    ) -> Result<(), UserError> {
        self.user_library.set_watched(user, target, watched).await
    }

    async fn title_states(
        &self,
        user: &UserId,
        titles: &[TitleId],
    ) -> Result<Vec<TitleState>, UserError> {
        self.user_library.title_states(user, titles).await
    }
}

impl<A, C, Se, L, U, Ul, D> DiscoveryService for AppState<A, C, Se, L, U, Ul, D>
where
    A: Sync,
    C: Sync,
    Se: Sync,
    L: Sync,
    U: Sync,
    Ul: Sync,
    D: DiscoveryService + Sync,
{
    async fn search(
        &self,
        user: &UserId,
        query: &str,
        types: &[SearchKind],
        page: PageRequest,
    ) -> Result<Page<SearchResult>, DiscoveryError> {
        self.discovery.search(user, query, types, page).await
    }

    async fn continue_watching(
        &self,
        user: &UserId,
    ) -> Result<Vec<ContinueWatchingItem>, DiscoveryError> {
        self.discovery.continue_watching(user).await
    }

    async fn now_playing(
        &self,
        sessions: Vec<PlaybackSession>,
    ) -> Result<Vec<NowPlaying>, DiscoveryError> {
        self.discovery.now_playing(sessions).await
    }

    async fn next_episodes(&self, user: &UserId) -> Result<Vec<Episode>, DiscoveryError> {
        self.discovery.next_episodes(user).await
    }

    async fn next_movies(&self, user: &UserId) -> Result<Vec<Movie>, DiscoveryError> {
        self.discovery.next_movies(user).await
    }

    async fn home_hubs(&self, user: &UserId) -> Result<Vec<Hub>, DiscoveryError> {
        self.discovery.home_hubs(user).await
    }
}

pub trait AppServices:
    AuthService
    + CatalogService
    + SessionService
    + LibraryService
    + UserService
    + UserLibraryService
    + DiscoveryService
    + Clone
    + Send
    + Sync
    + 'static
{
}

impl<T> AppServices for T where
    T: AuthService
        + CatalogService
        + SessionService
        + LibraryService
        + UserService
        + UserLibraryService
        + DiscoveryService
        + Clone
        + Send
        + Sync
        + 'static
{
}

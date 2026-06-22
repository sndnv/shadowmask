use std::future::Future;

use crate::catalog::{
    Collection, CollectionId, CollectionUpdate, Episode, EpisodeId, Movie, MovieId, NewCollection,
    Season, SeasonId, Series, SeriesId, TitleId, Version, VersionId,
};
use crate::common::{Page, PageRequest};
use crate::discovery::{ContinueWatchingItem, Hub, SearchResult};
use crate::error::{
    AuthError, CatalogError, DiscoveryError, LibraryError, SessionError, UserError,
};
use crate::library::{DuplicateCandidate, Library, LibraryId, ScanState, UnmatchedFile};
use crate::playback::{Favorite, PlaybackProgress, WatchHistory, WatchlistItem};
use crate::session::{
    HeartbeatAck, PlaybackSession, PlaybackState, Renegotiated, SessionId, SessionStarted,
    SessionUpdate, StartSessionRequest,
};
use crate::user::{
    AccessToken, DeviceRegistration, IssuedToken, LibraryAccess, NewUser, Principal, TokenPair,
    User, UserId, UserProfileUpdate,
};

pub trait AuthService {
    fn login(
        &self,
        username: &str,
        password: &str,
    ) -> impl Future<Output = Result<TokenPair, AuthError>> + Send;
    fn refresh(
        &self,
        refresh_token: &str,
    ) -> impl Future<Output = Result<AccessToken, AuthError>> + Send;
    fn redeem_link_code(
        &self,
        code: &str,
        device: DeviceRegistration,
    ) -> impl Future<Output = Result<IssuedToken, AuthError>> + Send;
    fn authenticate(
        &self,
        access_token: &str,
    ) -> impl Future<Output = Result<Principal, AuthError>> + Send;
}

pub trait CatalogService {
    fn collections(
        &self,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<Collection>, CatalogError>> + Send;
    fn collection(
        &self,
        id: &CollectionId,
    ) -> impl Future<Output = Result<Collection, CatalogError>> + Send;
    fn create_collection(
        &self,
        input: NewCollection,
    ) -> impl Future<Output = Result<Collection, CatalogError>> + Send;
    fn update_collection(
        &self,
        id: &CollectionId,
        update: CollectionUpdate,
    ) -> impl Future<Output = Result<Collection, CatalogError>> + Send;
    fn delete_collection(
        &self,
        id: &CollectionId,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send;
    fn movies(
        &self,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<Movie>, CatalogError>> + Send;
    fn movie(&self, id: &MovieId) -> impl Future<Output = Result<Movie, CatalogError>> + Send;
    fn series(
        &self,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<Series>, CatalogError>> + Send;
    fn series_detail(
        &self,
        id: &SeriesId,
    ) -> impl Future<Output = Result<Series, CatalogError>> + Send;
    fn seasons(
        &self,
        series: &SeriesId,
    ) -> impl Future<Output = Result<Vec<Season>, CatalogError>> + Send;
    fn season(&self, id: &SeasonId) -> impl Future<Output = Result<Season, CatalogError>> + Send;
    fn episodes(
        &self,
        season: &SeasonId,
    ) -> impl Future<Output = Result<Vec<Episode>, CatalogError>> + Send;
    fn episode(&self, id: &EpisodeId)
    -> impl Future<Output = Result<Episode, CatalogError>> + Send;
    fn versions(
        &self,
        title: &TitleId,
    ) -> impl Future<Output = Result<Vec<Version>, CatalogError>> + Send;
    fn library_versions(
        &self,
        library: &LibraryId,
    ) -> impl Future<Output = Result<Vec<Version>, CatalogError>> + Send;
}

pub trait SessionService {
    fn start(
        &self,
        user: &UserId,
        request: StartSessionRequest,
    ) -> impl Future<Output = Result<SessionStarted, SessionError>> + Send;
    fn heartbeat(
        &self,
        session: &SessionId,
        position_ms: u64,
        state: PlaybackState,
    ) -> impl Future<Output = Result<HeartbeatAck, SessionError>> + Send;
    fn seek(
        &self,
        session: &SessionId,
        position_ms: u64,
    ) -> impl Future<Output = Result<Renegotiated, SessionError>> + Send;
    fn update(
        &self,
        session: &SessionId,
        update: SessionUpdate,
    ) -> impl Future<Output = Result<Renegotiated, SessionError>> + Send;
    fn end(&self, session: &SessionId) -> impl Future<Output = Result<(), SessionError>> + Send;
    fn active_sessions(
        &self,
    ) -> impl Future<Output = Result<Vec<PlaybackSession>, SessionError>> + Send;
}

pub trait LibraryService {
    fn libraries(&self) -> impl Future<Output = Result<Vec<Library>, LibraryError>> + Send;
    fn library(&self, id: &LibraryId)
    -> impl Future<Output = Result<Library, LibraryError>> + Send;
    fn scan_state(
        &self,
        id: &LibraryId,
    ) -> impl Future<Output = Result<ScanState, LibraryError>> + Send;
    fn trigger_scan(&self, id: &LibraryId)
    -> impl Future<Output = Result<(), LibraryError>> + Send;
    fn unmatched(
        &self,
        id: &LibraryId,
    ) -> impl Future<Output = Result<Vec<UnmatchedFile>, LibraryError>> + Send;
    fn duplicates(
        &self,
        id: &LibraryId,
    ) -> impl Future<Output = Result<Vec<DuplicateCandidate>, LibraryError>> + Send;
}

pub trait UserService {
    fn create(&self, input: NewUser) -> impl Future<Output = Result<User, UserError>> + Send;
    fn get(&self, id: &UserId) -> impl Future<Output = Result<User, UserError>> + Send;
    fn list(&self) -> impl Future<Output = Result<Vec<User>, UserError>> + Send;
    fn update_profile(
        &self,
        id: &UserId,
        update: UserProfileUpdate,
    ) -> impl Future<Output = Result<User, UserError>> + Send;
    fn delete(&self, id: &UserId) -> impl Future<Output = Result<(), UserError>> + Send;
    fn library_access(
        &self,
        id: &UserId,
    ) -> impl Future<Output = Result<Vec<LibraryAccess>, UserError>> + Send;
    fn set_library_access(
        &self,
        id: &UserId,
        libraries: &[LibraryId],
    ) -> impl Future<Output = Result<(), UserError>> + Send;
}

pub trait UserLibraryService {
    fn watchlist(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<WatchlistItem>, UserError>> + Send;
    fn add_to_watchlist(
        &self,
        user: &UserId,
        title: &TitleId,
    ) -> impl Future<Output = Result<(), UserError>> + Send;
    fn remove_from_watchlist(
        &self,
        user: &UserId,
        title_id: &str,
    ) -> impl Future<Output = Result<(), UserError>> + Send;
    fn favorites(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<Favorite>, UserError>> + Send;
    fn add_favorite(
        &self,
        user: &UserId,
        title: &TitleId,
    ) -> impl Future<Output = Result<(), UserError>> + Send;
    fn remove_favorite(
        &self,
        user: &UserId,
        title_id: &str,
    ) -> impl Future<Output = Result<(), UserError>> + Send;
    fn history(
        &self,
        user: &UserId,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<WatchHistory>, UserError>> + Send;
    fn progress(
        &self,
        user: &UserId,
        version: &VersionId,
    ) -> impl Future<Output = Result<Option<PlaybackProgress>, UserError>> + Send;
}

pub trait DiscoveryService {
    fn search(
        &self,
        user: &UserId,
        query: &str,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<SearchResult>, DiscoveryError>> + Send;
    fn continue_watching(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<ContinueWatchingItem>, DiscoveryError>> + Send;
    fn next_episodes(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<Episode>, DiscoveryError>> + Send;
    fn next_movies(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<Movie>, DiscoveryError>> + Send;
    fn home_hubs(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<Hub>, DiscoveryError>> + Send;
}

use std::future::Future;

use crate::catalog::{
    Collection, CollectionId, CollectionUpdate, Episode, EpisodeId, Movie, MovieId, NewCollection,
    Season, SeasonId, Series, SeriesId, TitleId, Version, VersionDetail, VersionId,
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
    DeviceRegistration, IssuedToken, LibraryAccess, NewUser, Principal, TokenPair, User, UserId,
    UserProfileUpdate,
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
    ) -> impl Future<Output = Result<TokenPair, AuthError>> + Send;
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
        caller: &Principal,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<Collection>, CatalogError>> + Send;
    fn collection(
        &self,
        caller: &Principal,
        id: &CollectionId,
    ) -> impl Future<Output = Result<Collection, CatalogError>> + Send;
    fn create_collection(
        &self,
        caller: &Principal,
        input: NewCollection,
    ) -> impl Future<Output = Result<Collection, CatalogError>> + Send;
    fn update_collection(
        &self,
        caller: &Principal,
        id: &CollectionId,
        update: CollectionUpdate,
    ) -> impl Future<Output = Result<Collection, CatalogError>> + Send;
    fn delete_collection(
        &self,
        caller: &Principal,
        id: &CollectionId,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send;
    fn movies(
        &self,
        caller: &Principal,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<Movie>, CatalogError>> + Send;
    fn movie(
        &self,
        caller: &Principal,
        id: &MovieId,
    ) -> impl Future<Output = Result<Movie, CatalogError>> + Send;
    fn series(
        &self,
        caller: &Principal,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<Series>, CatalogError>> + Send;
    fn series_detail(
        &self,
        caller: &Principal,
        id: &SeriesId,
    ) -> impl Future<Output = Result<Series, CatalogError>> + Send;
    fn seasons(
        &self,
        caller: &Principal,
        series: &SeriesId,
    ) -> impl Future<Output = Result<Vec<Season>, CatalogError>> + Send;
    fn season(
        &self,
        caller: &Principal,
        id: &SeasonId,
    ) -> impl Future<Output = Result<Season, CatalogError>> + Send;
    fn episodes(
        &self,
        caller: &Principal,
        season: &SeasonId,
    ) -> impl Future<Output = Result<Vec<Episode>, CatalogError>> + Send;
    fn episode(
        &self,
        caller: &Principal,
        id: &EpisodeId,
    ) -> impl Future<Output = Result<Episode, CatalogError>> + Send;
    fn versions(
        &self,
        caller: &Principal,
        title: &TitleId,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<Version>, CatalogError>> + Send;
    fn library_versions(
        &self,
        caller: &Principal,
        library: &LibraryId,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<Version>, CatalogError>> + Send;
    fn version(
        &self,
        caller: &Principal,
        id: &VersionId,
    ) -> impl Future<Output = Result<VersionDetail, CatalogError>> + Send;
}

pub trait SessionService {
    fn start(
        &self,
        caller: &Principal,
        request: StartSessionRequest,
    ) -> impl Future<Output = Result<SessionStarted, SessionError>> + Send;
    fn heartbeat(
        &self,
        caller: &Principal,
        session: &SessionId,
        position_ms: u64,
        state: PlaybackState,
    ) -> impl Future<Output = Result<HeartbeatAck, SessionError>> + Send;
    fn seek(
        &self,
        caller: &Principal,
        session: &SessionId,
        position_ms: u64,
    ) -> impl Future<Output = Result<Renegotiated, SessionError>> + Send;
    fn update(
        &self,
        caller: &Principal,
        session: &SessionId,
        update: SessionUpdate,
    ) -> impl Future<Output = Result<Renegotiated, SessionError>> + Send;
    fn end(
        &self,
        caller: &Principal,
        session: &SessionId,
    ) -> impl Future<Output = Result<(), SessionError>> + Send;
    fn active_sessions(
        &self,
        caller: &Principal,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<PlaybackSession>, SessionError>> + Send;
}

pub trait LibraryService {
    fn libraries(
        &self,
        caller: &Principal,
    ) -> impl Future<Output = Result<Vec<Library>, LibraryError>> + Send;
    fn library(
        &self,
        caller: &Principal,
        id: &LibraryId,
    ) -> impl Future<Output = Result<Library, LibraryError>> + Send;
    fn scan_state(
        &self,
        caller: &Principal,
        id: &LibraryId,
    ) -> impl Future<Output = Result<ScanState, LibraryError>> + Send;
    fn trigger_scan(
        &self,
        caller: &Principal,
        id: &LibraryId,
    ) -> impl Future<Output = Result<(), LibraryError>> + Send;
    fn unmatched(
        &self,
        caller: &Principal,
        id: &LibraryId,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<UnmatchedFile>, LibraryError>> + Send;
    fn duplicates(
        &self,
        caller: &Principal,
        id: &LibraryId,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<DuplicateCandidate>, LibraryError>> + Send;
}

pub trait UserService {
    fn create(&self, input: NewUser) -> impl Future<Output = Result<User, UserError>> + Send;
    fn get(&self, id: &UserId) -> impl Future<Output = Result<User, UserError>> + Send;
    fn list(&self, page: PageRequest)
    -> impl Future<Output = Result<Page<User>, UserError>> + Send;
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

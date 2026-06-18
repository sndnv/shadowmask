use crate::catalog::{
    Collection, CollectionId, Episode, EpisodeId, Movie, MovieId, Season, SeasonId, Series,
    SeriesId, TitleId, Version, VersionId,
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
    async fn login(&self, username: &str, password: &str) -> Result<TokenPair, AuthError>;
    async fn refresh(&self, refresh_token: &str) -> Result<AccessToken, AuthError>;
    async fn redeem_link_code(
        &self,
        code: &str,
        device: DeviceRegistration,
    ) -> Result<IssuedToken, AuthError>;
    async fn authenticate(&self, access_token: &str) -> Result<Principal, AuthError>;
}

pub trait CatalogService {
    async fn collections(&self, page: PageRequest) -> Result<Page<Collection>, CatalogError>;
    async fn collection(&self, id: &CollectionId) -> Result<Collection, CatalogError>;
    async fn movies(&self, page: PageRequest) -> Result<Page<Movie>, CatalogError>;
    async fn movie(&self, id: &MovieId) -> Result<Movie, CatalogError>;
    async fn series(&self, page: PageRequest) -> Result<Page<Series>, CatalogError>;
    async fn series_detail(&self, id: &SeriesId) -> Result<Series, CatalogError>;
    async fn seasons(&self, series: &SeriesId) -> Result<Vec<Season>, CatalogError>;
    async fn episodes(&self, season: &SeasonId) -> Result<Vec<Episode>, CatalogError>;
    async fn episode(&self, id: &EpisodeId) -> Result<Episode, CatalogError>;
    async fn versions(&self, title: &TitleId) -> Result<Vec<Version>, CatalogError>;
}

pub trait SessionService {
    async fn start(
        &self,
        user: &UserId,
        request: StartSessionRequest,
    ) -> Result<SessionStarted, SessionError>;
    async fn heartbeat(
        &self,
        session: &SessionId,
        position_ms: u64,
        state: PlaybackState,
    ) -> Result<HeartbeatAck, SessionError>;
    async fn seek(
        &self,
        session: &SessionId,
        position_ms: u64,
    ) -> Result<Renegotiated, SessionError>;
    async fn update(
        &self,
        session: &SessionId,
        update: SessionUpdate,
    ) -> Result<Renegotiated, SessionError>;
    async fn end(&self, session: &SessionId) -> Result<(), SessionError>;
    async fn active_sessions(&self) -> Result<Vec<PlaybackSession>, SessionError>;
}

pub trait LibraryService {
    async fn libraries(&self) -> Result<Vec<Library>, LibraryError>;
    async fn library(&self, id: &LibraryId) -> Result<Library, LibraryError>;
    async fn scan_state(&self, id: &LibraryId) -> Result<ScanState, LibraryError>;
    async fn trigger_scan(&self, id: &LibraryId) -> Result<(), LibraryError>;
    async fn unmatched(&self, id: &LibraryId) -> Result<Vec<UnmatchedFile>, LibraryError>;
    async fn duplicates(&self, id: &LibraryId) -> Result<Vec<DuplicateCandidate>, LibraryError>;
}

pub trait UserService {
    async fn create(&self, input: NewUser) -> Result<User, UserError>;
    async fn get(&self, id: &UserId) -> Result<User, UserError>;
    async fn list(&self) -> Result<Vec<User>, UserError>;
    async fn update_profile(
        &self,
        id: &UserId,
        update: UserProfileUpdate,
    ) -> Result<User, UserError>;
    async fn delete(&self, id: &UserId) -> Result<(), UserError>;
    async fn library_access(&self, id: &UserId) -> Result<Vec<LibraryAccess>, UserError>;
    async fn set_library_access(
        &self,
        id: &UserId,
        libraries: &[LibraryId],
    ) -> Result<(), UserError>;
}

pub trait UserLibraryService {
    async fn watchlist(&self, user: &UserId) -> Result<Vec<WatchlistItem>, UserError>;
    async fn add_to_watchlist(&self, user: &UserId, title: &TitleId) -> Result<(), UserError>;
    async fn remove_from_watchlist(&self, user: &UserId, title: &TitleId) -> Result<(), UserError>;
    async fn favorites(&self, user: &UserId) -> Result<Vec<Favorite>, UserError>;
    async fn add_favorite(&self, user: &UserId, title: &TitleId) -> Result<(), UserError>;
    async fn remove_favorite(&self, user: &UserId, title: &TitleId) -> Result<(), UserError>;
    async fn history(
        &self,
        user: &UserId,
        page: PageRequest,
    ) -> Result<Page<WatchHistory>, UserError>;
    async fn progress(
        &self,
        user: &UserId,
        version: &VersionId,
    ) -> Result<Option<PlaybackProgress>, UserError>;
}

pub trait DiscoveryService {
    async fn search(
        &self,
        user: &UserId,
        query: &str,
        page: PageRequest,
    ) -> Result<Page<SearchResult>, DiscoveryError>;
    async fn continue_watching(
        &self,
        user: &UserId,
    ) -> Result<Vec<ContinueWatchingItem>, DiscoveryError>;
    async fn next_up(&self, user: &UserId) -> Result<Vec<Episode>, DiscoveryError>;
    async fn home_hubs(&self, user: &UserId) -> Result<Vec<Hub>, DiscoveryError>;
}

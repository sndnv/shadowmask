use std::future::Future;

use crate::catalog::{
    Collection, CollectionId, CollectionUpdate, Episode, EpisodeId, Movie, MovieDetail, MovieId,
    NewCollection, PersonProfile, Season, SeasonId, Series, SeriesDetail, SeriesId, TitleCard,
    TitleId, TitleListQuery, TitleRef, Version, VersionDetail, VersionId,
};
use crate::common::{Page, PageRequest};
use crate::discovery::{ContinueWatchingItem, Hub, SearchKind, SearchResult};
use crate::error::{
    AuthError, CatalogError, DiscoveryError, LibraryError, SessionError, UserError,
};
use crate::job::Job;
use crate::library::{
    DuplicateCandidate, DuplicateCandidateId, Library, LibraryId, LibraryUpdate, NewLibrary,
    ResolveCandidate, ResolveTarget, ScanState, UnmatchedFile, UnmatchedFileId,
};
use crate::metadata::{ExternalId, Genre, PersonId};
use crate::playback::{
    Favorite, PlaybackProgress, TitleState, WatchHistory, WatchTarget, WatchlistItem,
};
use crate::session::{
    HeartbeatAck, NowPlaying, PlaybackSession, PlaybackState, Renegotiated, SessionId,
    SessionStarted, SessionUpdate, StartSessionRequest,
};
use crate::user::{
    ApiToken, ApiTokenId, Device, DeviceId, DeviceRegistration, IssuedToken, LibraryAccess,
    NewUser, PendingLink, Principal, TokenPair, User, UserId, UserProfileUpdate,
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
    fn create_link_code(
        &self,
        caller: &Principal,
        user: Option<UserId>,
        ttl_secs: Option<i64>,
    ) -> impl Future<Output = Result<PendingLink, AuthError>> + Send;
    fn logout(&self, refresh_token: &str) -> impl Future<Output = Result<(), AuthError>> + Send;
    fn logout_all(&self, user: &UserId) -> impl Future<Output = Result<(), AuthError>> + Send;
    fn list_devices(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<Device>, AuthError>> + Send;
    fn list_api_tokens(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<ApiToken>, AuthError>> + Send;
    fn revoke_device(
        &self,
        user: &UserId,
        device: &DeviceId,
    ) -> impl Future<Output = Result<(), AuthError>> + Send;
    fn revoke_api_token(
        &self,
        user: &UserId,
        token: &ApiTokenId,
    ) -> impl Future<Output = Result<(), AuthError>> + Send;
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
        query: &TitleListQuery,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<Movie>, CatalogError>> + Send;
    fn movie(
        &self,
        caller: &Principal,
        id: &MovieId,
    ) -> impl Future<Output = Result<MovieDetail, CatalogError>> + Send;
    fn series(
        &self,
        caller: &Principal,
        query: &TitleListQuery,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<Series>, CatalogError>> + Send;
    fn series_detail(
        &self,
        caller: &Principal,
        id: &SeriesId,
    ) -> impl Future<Output = Result<SeriesDetail, CatalogError>> + Send;
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
    fn person(
        &self,
        caller: &Principal,
        id: &PersonId,
    ) -> impl Future<Output = Result<PersonProfile, CatalogError>> + Send;
    fn genres(
        &self,
        caller: &Principal,
    ) -> impl Future<Output = Result<Vec<Genre>, CatalogError>> + Send;
    fn title_cards(
        &self,
        caller: &Principal,
        ids: &[TitleId],
    ) -> impl Future<Output = Result<Vec<TitleCard>, CatalogError>> + Send;
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
    fn create_library(
        &self,
        caller: &Principal,
        input: NewLibrary,
    ) -> impl Future<Output = Result<Library, LibraryError>> + Send;
    fn update_library(
        &self,
        caller: &Principal,
        id: &LibraryId,
        update: LibraryUpdate,
    ) -> impl Future<Output = Result<Library, LibraryError>> + Send;
    fn delete_library(
        &self,
        caller: &Principal,
        id: &LibraryId,
    ) -> impl Future<Output = Result<(), LibraryError>> + Send;
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
    fn unmatched_candidates(
        &self,
        caller: &Principal,
        id: &LibraryId,
        unmatched: &UnmatchedFileId,
        query: Option<String>,
    ) -> impl Future<Output = Result<Vec<ResolveCandidate>, LibraryError>> + Send;
    fn resolve_unmatched(
        &self,
        caller: &Principal,
        id: &LibraryId,
        unmatched: &UnmatchedFileId,
        target: ResolveTarget,
    ) -> impl Future<Output = Result<(), LibraryError>> + Send;
    fn dismiss_duplicate(
        &self,
        caller: &Principal,
        id: &LibraryId,
        duplicate: &DuplicateCandidateId,
    ) -> impl Future<Output = Result<(), LibraryError>> + Send;
    fn resolve_duplicate(
        &self,
        caller: &Principal,
        id: &LibraryId,
        duplicate: &DuplicateCandidateId,
    ) -> impl Future<Output = Result<(), LibraryError>> + Send;
    fn reidentify(
        &self,
        caller: &Principal,
        title: TitleRef,
        external_id: Option<ExternalId>,
    ) -> impl Future<Output = Result<(), LibraryError>> + Send;
    fn jobs(
        &self,
        caller: &Principal,
    ) -> impl Future<Output = Result<Vec<Job>, LibraryError>> + Send;
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
    fn change_password(
        &self,
        actor: &Principal,
        target: &UserId,
        current: Option<&str>,
        new_password: &str,
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
    fn clear_progress(
        &self,
        user: &UserId,
        version: &VersionId,
    ) -> impl Future<Output = Result<(), UserError>> + Send;
    fn set_watched(
        &self,
        user: &UserId,
        target: &WatchTarget,
        watched: bool,
    ) -> impl Future<Output = Result<(), UserError>> + Send;
    fn title_states(
        &self,
        user: &UserId,
        titles: &[TitleId],
    ) -> impl Future<Output = Result<Vec<TitleState>, UserError>> + Send;
}

pub trait DiscoveryService {
    fn search(
        &self,
        user: &UserId,
        query: &str,
        types: &[SearchKind],
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<SearchResult>, DiscoveryError>> + Send;
    fn continue_watching(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<ContinueWatchingItem>, DiscoveryError>> + Send;
    fn now_playing(
        &self,
        sessions: Vec<PlaybackSession>,
    ) -> impl Future<Output = Result<Vec<NowPlaying>, DiscoveryError>> + Send;
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

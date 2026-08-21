use std::future::Future;

use crate::catalog::{
    Collection, CollectionDetail, CollectionId, CollectionUpdate, Episode, EpisodeCard,
    EpisodeEdit, EpisodeId, Movie, MovieDetail, MovieEdit, MovieId, NewCollection, PersonProfile,
    RandomScope, Season, SeasonCard, SeasonId, Series, SeriesDetail, SeriesEdit, SeriesId,
    TitleCard, TitleId, TitleKind, TitleListQuery, TitleRef, Version, VersionDetail, VersionId,
};
use crate::common::{Page, PageRequest};
use crate::discovery::{ContinueWatchingItem, Hub, SearchKind, SearchResult};
use crate::error::{
    AuthError, CatalogError, DiscoveryError, JobServiceError, LibraryError, SessionError, UserError,
};
use crate::job::{Job, JobId, JobNode, JobPage, JobQuery};
use crate::library::{
    DuplicateCandidate, DuplicateCandidateId, FetchInput, Library, LibraryId, LibraryUpdate,
    NewLibrary, ResolveCandidate, ResolveTarget, ScanState, UnmatchedFile, UnmatchedFileId,
};
use crate::media::SubtitleFileId;
use crate::metadata::{ExternalId, Genre, Person, PersonId};
use crate::playback::{
    Favorite, PlaybackProgress, TitleState, WatchHistory, WatchTarget, WatchedRollup, WatchlistItem,
};
use crate::session::{
    HeartbeatAck, NowPlaying, PlaybackSession, PlaybackState, Renegotiated, SessionId,
    SessionStartInput, SessionStarted, SessionUpdate,
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
    fn list_link_codes(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<PendingLink>, AuthError>> + Send;
    fn revoke_link_code(
        &self,
        user: &UserId,
        code: &str,
    ) -> impl Future<Output = Result<(), AuthError>> + Send;
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
    ) -> impl Future<Output = Result<CollectionDetail, CatalogError>> + Send;
    fn movie_collections(
        &self,
        caller: &Principal,
        movie: &MovieId,
    ) -> impl Future<Output = Result<Vec<CollectionDetail>, CatalogError>> + Send;
    fn people_cards(
        &self,
        caller: &Principal,
        ids: &[PersonId],
    ) -> impl Future<Output = Result<Vec<Person>, CatalogError>> + Send;
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
    ) -> impl Future<Output = Result<SeasonCard, CatalogError>> + Send;
    fn episodes(
        &self,
        caller: &Principal,
        season: &SeasonId,
    ) -> impl Future<Output = Result<Vec<Episode>, CatalogError>> + Send;
    fn episode(
        &self,
        caller: &Principal,
        id: &EpisodeId,
    ) -> impl Future<Output = Result<EpisodeCard, CatalogError>> + Send;
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
    fn all_versions(
        &self,
        caller: &Principal,
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
        kind: Option<TitleKind>,
    ) -> impl Future<Output = Result<Vec<Genre>, CatalogError>> + Send;
    fn title_cards(
        &self,
        caller: &Principal,
        ids: &[TitleId],
    ) -> impl Future<Output = Result<Vec<TitleCard>, CatalogError>> + Send;
    fn random(
        &self,
        caller: &Principal,
        scope: &RandomScope,
        query: &TitleListQuery,
    ) -> impl Future<Output = Result<VersionId, CatalogError>> + Send;
}

pub trait SessionService {
    fn start(
        &self,
        caller: &Principal,
        request: SessionStartInput,
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
    fn end_all_for_user(
        &self,
        user: &UserId,
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
    fn create_fetch(
        &self,
        caller: &Principal,
        library: &LibraryId,
        input: FetchInput,
    ) -> impl Future<Output = Result<(), LibraryError>> + Send;
    fn dismiss_duplicate(
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
        force: bool,
    ) -> impl Future<Output = Result<(), LibraryError>> + Send;
    fn edit_movie(
        &self,
        caller: &Principal,
        id: &MovieId,
        edit: MovieEdit,
    ) -> impl Future<Output = Result<Movie, LibraryError>> + Send;
    fn edit_series(
        &self,
        caller: &Principal,
        id: &SeriesId,
        edit: SeriesEdit,
    ) -> impl Future<Output = Result<Series, LibraryError>> + Send;
    fn edit_episode(
        &self,
        caller: &Principal,
        id: &EpisodeId,
        edit: EpisodeEdit,
    ) -> impl Future<Output = Result<Episode, LibraryError>> + Send;
    fn refresh_library_metadata(
        &self,
        caller: &Principal,
        id: &LibraryId,
    ) -> impl Future<Output = Result<(), LibraryError>> + Send;
    fn refresh_person(
        &self,
        caller: &Principal,
        id: &PersonId,
    ) -> impl Future<Output = Result<(), LibraryError>> + Send;
    fn relink_version(
        &self,
        caller: &Principal,
        version: &VersionId,
        target: ResolveTarget,
    ) -> impl Future<Output = Result<(), LibraryError>> + Send;
    fn relink_series(
        &self,
        caller: &Principal,
        series: &SeriesId,
        target: ResolveTarget,
    ) -> impl Future<Output = Result<(), LibraryError>> + Send;
    fn delete_version(
        &self,
        caller: &Principal,
        version: &VersionId,
    ) -> impl Future<Output = Result<(), LibraryError>> + Send;
    fn delete_movie(
        &self,
        caller: &Principal,
        movie: &MovieId,
    ) -> impl Future<Output = Result<(), LibraryError>> + Send;
    fn delete_series(
        &self,
        caller: &Principal,
        series: &SeriesId,
    ) -> impl Future<Output = Result<(), LibraryError>> + Send;
    fn delete_season(
        &self,
        caller: &Principal,
        season: &SeasonId,
    ) -> impl Future<Output = Result<(), LibraryError>> + Send;
    fn delete_episode(
        &self,
        caller: &Principal,
        episode: &EpisodeId,
    ) -> impl Future<Output = Result<(), LibraryError>> + Send;
    fn trigger_transcription(
        &self,
        caller: &Principal,
        version: &VersionId,
        audio_track_index: Option<u32>,
        source_language: Option<String>,
    ) -> impl Future<Output = Result<(), LibraryError>> + Send;
    fn trigger_translation(
        &self,
        caller: &Principal,
        version: &VersionId,
        source_subtitle: &SubtitleFileId,
        target_language: String,
    ) -> impl Future<Output = Result<(), LibraryError>> + Send;
    fn trigger_upscale(
        &self,
        caller: &Principal,
        version: &VersionId,
        target_height: u32,
    ) -> impl Future<Output = Result<(), LibraryError>> + Send;
    fn trigger_combine(
        &self,
        caller: &Principal,
        version: &VersionId,
        top: &SubtitleFileId,
        bottom: &SubtitleFileId,
    ) -> impl Future<Output = Result<(), LibraryError>> + Send;
}

pub trait JobService {
    fn jobs(
        &self,
        caller: &Principal,
        query: &JobQuery,
        page: PageRequest,
    ) -> impl Future<Output = Result<JobPage, JobServiceError>> + Send;
    fn job(
        &self,
        caller: &Principal,
        id: &JobId,
    ) -> impl Future<Output = Result<Option<Job>, JobServiceError>> + Send;
    fn job_descendants(
        &self,
        caller: &Principal,
        id: &JobId,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<JobNode>, JobServiceError>> + Send;
    fn cancel_job(
        &self,
        caller: &Principal,
        id: &JobId,
    ) -> impl Future<Output = Result<(), JobServiceError>> + Send;
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
    fn delete(
        &self,
        caller: &Principal,
        id: &UserId,
    ) -> impl Future<Output = Result<(), UserError>> + Send;
    fn set_active(
        &self,
        caller: &Principal,
        id: &UserId,
        active: bool,
    ) -> impl Future<Output = Result<User, UserError>> + Send;
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
    fn remove_from_history(
        &self,
        user: &UserId,
        title_id: &str,
    ) -> impl Future<Output = Result<(), UserError>> + Send;
    fn clear_history(&self, user: &UserId) -> impl Future<Output = Result<(), UserError>> + Send;
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
    fn watched_rollups(
        &self,
        user: &UserId,
        targets: &[WatchTarget],
    ) -> impl Future<Output = Result<Vec<WatchedRollup>, UserError>> + Send;
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
    ) -> impl Future<Output = Result<Vec<EpisodeCard>, DiscoveryError>> + Send;
    fn next_movies(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<Movie>, DiscoveryError>> + Send;
    fn home_hubs(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<Hub>, DiscoveryError>> + Send;
}

use std::collections::{HashMap, HashSet};
use std::future::Future;

use jiff::Timestamp;

use crate::catalog::{
    ArtworkId, ArtworkOwner, ArtworkRef, Collection, CollectionId, Episode, EpisodeContext,
    EpisodeId, Movie, MovieDetail, MovieId, RandomScope, Season, SeasonId, Series, SeriesDetail,
    SeriesId, TitleId, TitleKind, TitleListFilter, TitleRef, Version, VersionDetail, VersionId,
};
use crate::common::{Page, PageRequest};
use crate::discovery::{SearchKind, SearchResult};
use crate::error::RepositoryError;
use crate::job::{Job, JobId, JobKind, JobNode, JobQuery, ReclaimOutcome};
use crate::library::{
    DuplicateCandidate, DuplicateCandidateId, Library, LibraryId, ResolutionStatus, ScanState,
    UnmatchedFile, UnmatchedFileId,
};
use crate::media::{
    AudioTrack, Chapter, EmbeddedSubtitleTrack, SubtitleFile, TrickplayAsset, VideoTrack,
};
use crate::metadata::{Credit, Genre, Person, PersonId, TitleEnrichment};
use crate::playback::{
    Favorite, PlaybackProgress, SubtitleTrackRef, UserSubtitleOffset, WatchHistory, WatchlistItem,
};
use crate::session::{PlaybackSession, SessionId};
use crate::user::{
    ApiToken, ApiTokenId, AuthSession, AuthSessionId, Device, DeviceId, LibraryAccess, PendingLink,
    User, UserId,
};

pub trait CatalogRepository {
    fn list_movies(
        &self,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<Movie>, RepositoryError>> + Send;
    fn get_movie(
        &self,
        id: &MovieId,
    ) -> impl Future<Output = Result<Option<Movie>, RepositoryError>> + Send;
    fn movies_by_ids(
        &self,
        ids: &[MovieId],
    ) -> impl Future<Output = Result<Vec<Movie>, RepositoryError>> + Send;
    fn series_versions(
        &self,
        series: &SeriesId,
    ) -> impl Future<Output = Result<Vec<Version>, RepositoryError>> + Send;
    fn series_by_ids(
        &self,
        ids: &[SeriesId],
    ) -> impl Future<Output = Result<Vec<Series>, RepositoryError>> + Send;
    fn list_series(
        &self,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<Series>, RepositoryError>> + Send;
    fn get_series(
        &self,
        id: &SeriesId,
    ) -> impl Future<Output = Result<Option<Series>, RepositoryError>> + Send;
    fn list_seasons(
        &self,
        series: &SeriesId,
    ) -> impl Future<Output = Result<Vec<Season>, RepositoryError>> + Send;
    fn list_episodes(
        &self,
        season: &SeasonId,
    ) -> impl Future<Output = Result<Vec<Episode>, RepositoryError>> + Send;
    fn episode_ids_for_series(
        &self,
        series: &[SeriesId],
    ) -> impl Future<Output = Result<HashMap<SeriesId, Vec<EpisodeId>>, RepositoryError>> + Send;
    fn episode_ids_for_seasons(
        &self,
        seasons: &[SeasonId],
    ) -> impl Future<Output = Result<HashMap<SeasonId, Vec<EpisodeId>>, RepositoryError>> + Send;
    fn recent_episodes(
        &self,
        filter: &TitleListFilter,
        limit: u32,
    ) -> impl Future<Output = Result<Vec<EpisodeContext>, RepositoryError>> + Send;
    fn visible_movies(
        &self,
        ids: &[MovieId],
        filter: &TitleListFilter,
    ) -> impl Future<Output = Result<Vec<Movie>, RepositoryError>> + Send;
    fn visible_episodes(
        &self,
        ids: &[EpisodeId],
        filter: &TitleListFilter,
    ) -> impl Future<Output = Result<Vec<EpisodeContext>, RepositoryError>> + Send;
    fn next_episode_in_series(
        &self,
        series: &SeriesId,
        after_season: u16,
        after_number: u16,
        filter: &TitleListFilter,
    ) -> impl Future<Output = Result<Option<EpisodeContext>, RepositoryError>> + Send;
    fn get_episode(
        &self,
        id: &EpisodeId,
    ) -> impl Future<Output = Result<Option<Episode>, RepositoryError>> + Send;
    fn list_collections(
        &self,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<Collection>, RepositoryError>> + Send;
    fn get_collection(
        &self,
        id: &CollectionId,
    ) -> impl Future<Output = Result<Option<Collection>, RepositoryError>> + Send;
    fn upsert_collection(
        &self,
        collection: Collection,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn delete_collection(
        &self,
        id: &CollectionId,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn collections_of_movie(
        &self,
        movie: &MovieId,
    ) -> impl Future<Output = Result<Vec<CollectionId>, RepositoryError>> + Send;
    fn get_season(
        &self,
        id: &SeasonId,
    ) -> impl Future<Output = Result<Option<Season>, RepositoryError>> + Send;
    fn list_versions(
        &self,
        title: &TitleId,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<Version>, RepositoryError>> + Send;
    fn list_library_versions(
        &self,
        library: &LibraryId,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<Version>, RepositoryError>> + Send;
    fn list_all_versions(
        &self,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<Version>, RepositoryError>> + Send;
    fn version_detail(
        &self,
        id: &VersionId,
    ) -> impl Future<Output = Result<Option<VersionDetail>, RepositoryError>> + Send;
    fn get_version(
        &self,
        id: &VersionId,
    ) -> impl Future<Output = Result<Option<Version>, RepositoryError>> + Send;
    fn delete_version(
        &self,
        id: &VersionId,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn delete_movie(
        &self,
        id: &MovieId,
    ) -> impl Future<Output = Result<bool, RepositoryError>> + Send;
    fn delete_series(
        &self,
        id: &SeriesId,
    ) -> impl Future<Output = Result<bool, RepositoryError>> + Send;
    fn delete_season(
        &self,
        id: &SeasonId,
    ) -> impl Future<Output = Result<bool, RepositoryError>> + Send;
    fn delete_episode(
        &self,
        id: &EpisodeId,
    ) -> impl Future<Output = Result<bool, RepositoryError>> + Send;
    fn upsert_movie(
        &self,
        movie: Movie,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn upsert_series(
        &self,
        series: Series,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn upsert_season(
        &self,
        season: Season,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn upsert_episode(
        &self,
        episode: Episode,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn upsert_version(
        &self,
        version: Version,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn reconcile_library_versions(
        &self,
        library: &LibraryId,
        present_paths: &[String],
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn set_artwork(
        &self,
        owner: &ArtworkOwner,
        refs: &[ArtworkRef],
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn list_artwork(
        &self,
        owner: &ArtworkOwner,
    ) -> impl Future<Output = Result<Vec<ArtworkRef>, RepositoryError>> + Send;
    fn all_artwork_ids(
        &self,
    ) -> impl Future<Output = Result<Vec<ArtworkId>, RepositoryError>> + Send;
    fn all_version_ids(
        &self,
    ) -> impl Future<Output = Result<Vec<VersionId>, RepositoryError>> + Send;
    fn live_artwork_ids(
        &self,
        ids: &[String],
    ) -> impl Future<Output = Result<HashSet<String>, RepositoryError>> + Send;
    fn live_version_ids(
        &self,
        ids: &[String],
    ) -> impl Future<Output = Result<HashSet<String>, RepositoryError>> + Send;
    fn live_artwork_paths(
        &self,
        ids: &[String],
    ) -> impl Future<Output = Result<HashSet<String>, RepositoryError>> + Send;
    fn live_subtitle_paths(
        &self,
        ids: &[String],
    ) -> impl Future<Output = Result<HashSet<String>, RepositoryError>> + Send;
    fn live_trickplay_paths(
        &self,
        ids: &[String],
    ) -> impl Future<Output = Result<HashSet<String>, RepositoryError>> + Send;
    fn set_version_tracks(
        &self,
        version: &VersionId,
        video: &[VideoTrack],
        audio: &[AudioTrack],
        subtitles: &[EmbeddedSubtitleTrack],
        chapters: &[Chapter],
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn set_trickplay(
        &self,
        version: &VersionId,
        assets: &[TrickplayAsset],
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn set_subtitle_files(
        &self,
        version: &VersionId,
        files: &[SubtitleFile],
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn add_subtitle_file(
        &self,
        version: &VersionId,
        file: &SubtitleFile,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn upsert_person(
        &self,
        person: Person,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn get_person(
        &self,
        id: &PersonId,
    ) -> impl Future<Output = Result<Option<Person>, RepositoryError>> + Send;
    fn set_title_enrichment(
        &self,
        owner: &TitleRef,
        enrichment: &TitleEnrichment,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn movie_detail(
        &self,
        id: &MovieId,
    ) -> impl Future<Output = Result<Option<MovieDetail>, RepositoryError>> + Send;
    fn series_detail(
        &self,
        id: &SeriesId,
    ) -> impl Future<Output = Result<Option<SeriesDetail>, RepositoryError>> + Send;
    fn filmography(
        &self,
        id: &PersonId,
    ) -> impl Future<Output = Result<Vec<Credit>, RepositoryError>> + Send;
    fn list_genres(
        &self,
        kind: Option<TitleKind>,
    ) -> impl Future<Output = Result<Vec<Genre>, RepositoryError>> + Send;
    fn list_movies_filtered(
        &self,
        filter: &TitleListFilter,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<Movie>, RepositoryError>> + Send;
    fn list_series_filtered(
        &self,
        filter: &TitleListFilter,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<Series>, RepositoryError>> + Send;
    fn random_playable_title(
        &self,
        scope: &RandomScope,
        filter: &TitleListFilter,
    ) -> impl Future<Output = Result<Option<TitleId>, RepositoryError>> + Send;
}

pub trait VersionCatalog {
    fn version_detail(
        &self,
        id: &VersionId,
    ) -> impl Future<Output = Result<Option<VersionDetail>, RepositoryError>> + Send;
    fn get_version(
        &self,
        id: &VersionId,
    ) -> impl Future<Output = Result<Option<Version>, RepositoryError>> + Send;
}

impl<T: CatalogRepository + Send + Sync> VersionCatalog for T {
    async fn version_detail(
        &self,
        id: &VersionId,
    ) -> Result<Option<VersionDetail>, RepositoryError> {
        CatalogRepository::version_detail(self, id).await
    }

    async fn get_version(&self, id: &VersionId) -> Result<Option<Version>, RepositoryError> {
        CatalogRepository::get_version(self, id).await
    }
}

pub trait LibraryRepository {
    fn list(&self) -> impl Future<Output = Result<Vec<Library>, RepositoryError>> + Send;
    fn get(
        &self,
        id: &LibraryId,
    ) -> impl Future<Output = Result<Option<Library>, RepositoryError>> + Send;
    fn scan_state(
        &self,
        id: &LibraryId,
    ) -> impl Future<Output = Result<Option<ScanState>, RepositoryError>> + Send;
    fn save_scan_state(
        &self,
        state: ScanState,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn upsert(&self, library: Library) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn delete(&self, id: &LibraryId) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn list_unmatched(
        &self,
        id: &LibraryId,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<UnmatchedFile>, RepositoryError>> + Send;
    fn list_duplicates(
        &self,
        id: &LibraryId,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<DuplicateCandidate>, RepositoryError>> + Send;
    fn get_unmatched(
        &self,
        id: &UnmatchedFileId,
    ) -> impl Future<Output = Result<Option<UnmatchedFile>, RepositoryError>> + Send;
    fn insert_unmatched(
        &self,
        file: UnmatchedFile,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn insert_duplicate(
        &self,
        library: &LibraryId,
        duplicate: DuplicateCandidate,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn set_unmatched_status(
        &self,
        id: &UnmatchedFileId,
        status: ResolutionStatus,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn set_duplicate_status(
        &self,
        id: &DuplicateCandidateId,
        status: ResolutionStatus,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn reconcile_duplicates(
        &self,
        library: &LibraryId,
        detected: &[DuplicateCandidateId],
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn reconcile_unmatched(
        &self,
        library: &LibraryId,
        present_paths: &[String],
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
}

pub trait UserRepository {
    fn create(&self, user: User) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn get(
        &self,
        id: &UserId,
    ) -> impl Future<Output = Result<Option<User>, RepositoryError>> + Send;
    fn find_by_username(
        &self,
        username: &str,
    ) -> impl Future<Output = Result<Option<User>, RepositoryError>> + Send;
    fn list(
        &self,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<User>, RepositoryError>> + Send;
    fn update(&self, user: User) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn delete(&self, id: &UserId) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn list_library_access(
        &self,
        id: &UserId,
    ) -> impl Future<Output = Result<Vec<LibraryAccess>, RepositoryError>> + Send;
    fn set_library_access(
        &self,
        id: &UserId,
        libraries: &[LibraryId],
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn revoke_library_access(
        &self,
        library: &LibraryId,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
}

pub trait UserDataStore {
    fn purge(&self, user: &UserId) -> impl Future<Output = Result<(), RepositoryError>> + Send;
}

pub trait ProgressRepository {
    fn get(
        &self,
        user: &UserId,
        version: &VersionId,
    ) -> impl Future<Output = Result<Option<PlaybackProgress>, RepositoryError>> + Send;
    fn upsert(
        &self,
        progress: PlaybackProgress,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn delete(
        &self,
        user: &UserId,
        version: &VersionId,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn list_in_progress(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<PlaybackProgress>, RepositoryError>> + Send;
    fn record_view(
        &self,
        user: &UserId,
        title: &TitleId,
        at: Timestamp,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn set_watched_flags(
        &self,
        user: &UserId,
        title: &TitleId,
        watched: bool,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn delete_history(
        &self,
        user: &UserId,
        title_id: &str,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn clear_history(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn watched_state(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<WatchHistory>, RepositoryError>> + Send;
    fn history(
        &self,
        user: &UserId,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<WatchHistory>, RepositoryError>> + Send;
}

pub trait PreferencesRepository {
    fn list_watchlist(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<WatchlistItem>, RepositoryError>> + Send;
    fn add_watchlist(
        &self,
        item: WatchlistItem,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn remove_watchlist(
        &self,
        user: &UserId,
        title_id: &str,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn list_favorites(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<Favorite>, RepositoryError>> + Send;
    fn add_favorite(
        &self,
        item: Favorite,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn remove_favorite(
        &self,
        user: &UserId,
        title_id: &str,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn get_subtitle_offset(
        &self,
        user: &UserId,
        version: &VersionId,
        subtitle: &SubtitleTrackRef,
    ) -> impl Future<Output = Result<Option<UserSubtitleOffset>, RepositoryError>> + Send;
    fn set_subtitle_offset(
        &self,
        offset: UserSubtitleOffset,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
}

pub trait SearchIndex {
    fn search(
        &self,
        query: &str,
        types: &[SearchKind],
        filter: &TitleListFilter,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<SearchResult>, RepositoryError>> + Send;
    fn rebuild(&self) -> impl Future<Output = Result<(), RepositoryError>> + Send;
}

pub trait SessionRegistry {
    fn insert(
        &self,
        session: PlaybackSession,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn get(
        &self,
        id: &SessionId,
    ) -> impl Future<Output = Result<Option<PlaybackSession>, RepositoryError>> + Send;
    fn list_for_user(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<PlaybackSession>, RepositoryError>> + Send;
    fn list_all(
        &self,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<PlaybackSession>, RepositoryError>> + Send;
    fn remove(&self, id: &SessionId) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn remove_idle(
        &self,
        cutoff: Timestamp,
    ) -> impl Future<Output = Result<usize, RepositoryError>> + Send;
}

pub trait JobRepository {
    fn enqueue(&self, job: Job) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn claim_ready(
        &self,
        now: Timestamp,
        limit: usize,
        kinds: Vec<JobKind>,
    ) -> impl Future<Output = Result<Vec<Job>, RepositoryError>> + Send;
    fn reclaim_running(
        &self,
        now: Timestamp,
        max_attempts: u32,
    ) -> impl Future<Output = Result<ReclaimOutcome, RepositoryError>> + Send;
    fn update(&self, job: Job) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn get(&self, id: &JobId) -> impl Future<Output = Result<Option<Job>, RepositoryError>> + Send;
    fn list(&self) -> impl Future<Output = Result<Vec<Job>, RepositoryError>> + Send;
    fn list_page(
        &self,
        query: &JobQuery,
        page: PageRequest,
    ) -> impl Future<Output = Result<Vec<Job>, RepositoryError>> + Send;
    fn count(&self, query: &JobQuery) -> impl Future<Output = Result<u64, RepositoryError>> + Send;
    fn list_descendants(
        &self,
        root: &JobId,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<JobNode>, RepositoryError>> + Send;
    fn cancel(
        &self,
        id: &JobId,
        now: Timestamp,
    ) -> impl Future<Output = Result<bool, RepositoryError>> + Send;
    fn delete_finished_before(
        &self,
        cutoff: Timestamp,
    ) -> impl Future<Output = Result<Vec<JobId>, RepositoryError>> + Send;
}

pub trait AuthTokenRepository {
    fn store_refresh(
        &self,
        session: AuthSession,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn find_refresh(
        &self,
        jti: &AuthSessionId,
    ) -> impl Future<Output = Result<Option<AuthSession>, RepositoryError>> + Send;
    fn revoke_refresh(
        &self,
        jti: &AuthSessionId,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn revoke_all_for_user(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn store_link_code(
        &self,
        link: PendingLink,
        now: Timestamp,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn redeem_link_code(
        &self,
        code: &str,
        now: Timestamp,
    ) -> impl Future<Output = Result<Option<PendingLink>, RepositoryError>> + Send;
    fn list_link_codes(
        &self,
        user: &UserId,
        now: Timestamp,
    ) -> impl Future<Output = Result<Vec<PendingLink>, RepositoryError>> + Send;
    fn delete_link_code(
        &self,
        code: &str,
        user: &UserId,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn upsert_device(
        &self,
        device: Device,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn get_device(
        &self,
        id: &DeviceId,
    ) -> impl Future<Output = Result<Option<Device>, RepositoryError>> + Send;
    fn store_api_token(
        &self,
        token: ApiToken,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn find_api_token_by_hash(
        &self,
        hash: &str,
    ) -> impl Future<Output = Result<Option<ApiToken>, RepositoryError>> + Send;
    fn touch_api_token(
        &self,
        id: &ApiTokenId,
        now: Timestamp,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn list_devices(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<Device>, RepositoryError>> + Send;
    fn list_api_tokens(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<ApiToken>, RepositoryError>> + Send;
    fn delete_device(
        &self,
        id: &DeviceId,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn revoke_api_token(
        &self,
        id: &ApiTokenId,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn purge_user(&self, user: &UserId)
    -> impl Future<Output = Result<(), RepositoryError>> + Send;
}

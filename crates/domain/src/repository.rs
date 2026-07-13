use std::future::Future;

use jiff::Timestamp;

use crate::catalog::{
    ArtworkOwner, ArtworkRef, Collection, CollectionId, Episode, EpisodeId, Movie, MovieDetail,
    MovieId, Season, SeasonId, Series, SeriesDetail, SeriesId, TitleId, TitleKind, TitleRef,
    Version, VersionDetail, VersionId,
};
use crate::common::{Page, PageRequest};
use crate::discovery::{SearchKind, SearchResult};
use crate::error::RepositoryError;
use crate::job::{Job, JobId};
use crate::library::{
    DuplicateCandidate, DuplicateCandidateId, Library, LibraryId, ResolutionStatus, ScanState,
    UnmatchedFile, UnmatchedFileId,
};
use crate::media::{AudioTrack, Chapter, EmbeddedSubtitleTrack, TrickplayAsset, VideoTrack};
use crate::metadata::{Credit, Genre, GenreId, Person, PersonId, TitleEnrichment};
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
    fn version_detail(
        &self,
        id: &VersionId,
    ) -> impl Future<Output = Result<Option<VersionDetail>, RepositoryError>> + Send;
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
    fn set_artwork(
        &self,
        owner: &ArtworkOwner,
        refs: &[ArtworkRef],
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn list_artwork(
        &self,
        owner: &ArtworkOwner,
    ) -> impl Future<Output = Result<Vec<ArtworkRef>, RepositoryError>> + Send;
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
    fn list_genres(&self) -> impl Future<Output = Result<Vec<Genre>, RepositoryError>> + Send;
    fn list_movies_by_genre(
        &self,
        genre: &GenreId,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<Movie>, RepositoryError>> + Send;
    fn list_series_by_genre(
        &self,
        genre: &GenreId,
        page: PageRequest,
    ) -> impl Future<Output = Result<Page<Series>, RepositoryError>> + Send;
    fn titles_in_library(
        &self,
        kind: TitleKind,
        library: &LibraryId,
    ) -> impl Future<Output = Result<Vec<String>, RepositoryError>> + Send;
}

pub trait VersionCatalog {
    fn version_detail(
        &self,
        id: &VersionId,
    ) -> impl Future<Output = Result<Option<VersionDetail>, RepositoryError>> + Send;
}

impl<T: CatalogRepository + Send + Sync> VersionCatalog for T {
    async fn version_detail(
        &self,
        id: &VersionId,
    ) -> Result<Option<VersionDetail>, RepositoryError> {
        CatalogRepository::version_detail(self, id).await
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
    fn record_history(
        &self,
        history: WatchHistory,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
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
}

pub trait JobRepository {
    fn enqueue(&self, job: Job) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn claim_ready(
        &self,
        now: Timestamp,
        limit: usize,
    ) -> impl Future<Output = Result<Vec<Job>, RepositoryError>> + Send;
    fn update(&self, job: Job) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn get(&self, id: &JobId) -> impl Future<Output = Result<Option<Job>, RepositoryError>> + Send;
    fn list(&self) -> impl Future<Output = Result<Vec<Job>, RepositoryError>> + Send;
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
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn redeem_link_code(
        &self,
        code: &str,
        now: Timestamp,
    ) -> impl Future<Output = Result<Option<PendingLink>, RepositoryError>> + Send;
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
}

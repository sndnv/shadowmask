use std::future::Future;

use jiff::Timestamp;

use crate::catalog::{
    Collection, CollectionId, Episode, EpisodeId, Movie, MovieId, Season, SeasonId, Series,
    SeriesId, TitleId, Version, VersionDetail, VersionId,
};
use crate::common::{Page, PageRequest};
use crate::error::RepositoryError;
use crate::job::{Job, JobId};
use crate::library::{DuplicateCandidate, Library, LibraryId, ScanState, UnmatchedFile};
use crate::playback::{
    Favorite, PlaybackProgress, SubtitleTrackRef, UserSubtitleOffset, WatchHistory, WatchlistItem,
};
use crate::session::{PlaybackSession, SessionId};
use crate::user::{LibraryAccess, User, UserId};

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

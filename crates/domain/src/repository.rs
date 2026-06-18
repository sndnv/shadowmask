use crate::catalog::{
    Collection, CollectionId, Episode, EpisodeId, Movie, MovieId, Season, SeasonId, Series,
    SeriesId, TitleId, Version, VersionId,
};
use crate::common::{Page, PageRequest};
use crate::error::RepositoryError;
use crate::library::{DuplicateCandidate, Library, LibraryId, ScanState, UnmatchedFile};
use crate::playback::{
    Favorite, PlaybackProgress, SubtitleTrackRef, UserSubtitleOffset, WatchHistory, WatchlistItem,
};
use crate::session::{PlaybackSession, SessionId};
use crate::user::{LibraryAccess, User, UserId};

pub trait CatalogRepository {
    async fn list_movies(&self, page: PageRequest) -> Result<Page<Movie>, RepositoryError>;
    async fn get_movie(&self, id: &MovieId) -> Result<Option<Movie>, RepositoryError>;
    async fn list_series(&self, page: PageRequest) -> Result<Page<Series>, RepositoryError>;
    async fn get_series(&self, id: &SeriesId) -> Result<Option<Series>, RepositoryError>;
    async fn list_seasons(&self, series: &SeriesId) -> Result<Vec<Season>, RepositoryError>;
    async fn list_episodes(&self, season: &SeasonId) -> Result<Vec<Episode>, RepositoryError>;
    async fn get_episode(&self, id: &EpisodeId) -> Result<Option<Episode>, RepositoryError>;
    async fn list_collections(
        &self,
        page: PageRequest,
    ) -> Result<Page<Collection>, RepositoryError>;
    async fn get_collection(
        &self,
        id: &CollectionId,
    ) -> Result<Option<Collection>, RepositoryError>;
    async fn list_versions(&self, title: &TitleId) -> Result<Vec<Version>, RepositoryError>;
}

pub trait LibraryRepository {
    async fn list(&self) -> Result<Vec<Library>, RepositoryError>;
    async fn get(&self, id: &LibraryId) -> Result<Option<Library>, RepositoryError>;
    async fn scan_state(&self, id: &LibraryId) -> Result<Option<ScanState>, RepositoryError>;
    async fn list_unmatched(&self, id: &LibraryId) -> Result<Vec<UnmatchedFile>, RepositoryError>;
    async fn list_duplicates(
        &self,
        id: &LibraryId,
    ) -> Result<Vec<DuplicateCandidate>, RepositoryError>;
}

pub trait UserRepository {
    async fn create(&self, user: User) -> Result<(), RepositoryError>;
    async fn get(&self, id: &UserId) -> Result<Option<User>, RepositoryError>;
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, RepositoryError>;
    async fn list(&self) -> Result<Vec<User>, RepositoryError>;
    async fn update(&self, user: User) -> Result<(), RepositoryError>;
    async fn delete(&self, id: &UserId) -> Result<(), RepositoryError>;
    async fn list_library_access(&self, id: &UserId)
    -> Result<Vec<LibraryAccess>, RepositoryError>;
    async fn set_library_access(
        &self,
        id: &UserId,
        libraries: &[LibraryId],
    ) -> Result<(), RepositoryError>;
}

pub trait ProgressRepository {
    async fn get(
        &self,
        user: &UserId,
        version: &VersionId,
    ) -> Result<Option<PlaybackProgress>, RepositoryError>;
    async fn upsert(&self, progress: PlaybackProgress) -> Result<(), RepositoryError>;
    async fn list_in_progress(
        &self,
        user: &UserId,
    ) -> Result<Vec<PlaybackProgress>, RepositoryError>;
    async fn record_history(&self, history: WatchHistory) -> Result<(), RepositoryError>;
    async fn history(
        &self,
        user: &UserId,
        page: PageRequest,
    ) -> Result<Page<WatchHistory>, RepositoryError>;
}

pub trait PreferencesRepository {
    async fn list_watchlist(&self, user: &UserId) -> Result<Vec<WatchlistItem>, RepositoryError>;
    async fn add_watchlist(&self, item: WatchlistItem) -> Result<(), RepositoryError>;
    async fn remove_watchlist(&self, user: &UserId, title: &TitleId)
    -> Result<(), RepositoryError>;
    async fn list_favorites(&self, user: &UserId) -> Result<Vec<Favorite>, RepositoryError>;
    async fn add_favorite(&self, item: Favorite) -> Result<(), RepositoryError>;
    async fn remove_favorite(&self, user: &UserId, title: &TitleId) -> Result<(), RepositoryError>;
    async fn get_subtitle_offset(
        &self,
        user: &UserId,
        version: &VersionId,
        subtitle: &SubtitleTrackRef,
    ) -> Result<Option<UserSubtitleOffset>, RepositoryError>;
    async fn set_subtitle_offset(&self, offset: UserSubtitleOffset) -> Result<(), RepositoryError>;
}

pub trait SessionRegistry {
    async fn insert(&self, session: PlaybackSession) -> Result<(), RepositoryError>;
    async fn get(&self, id: &SessionId) -> Result<Option<PlaybackSession>, RepositoryError>;
    async fn list_for_user(&self, user: &UserId) -> Result<Vec<PlaybackSession>, RepositoryError>;
    async fn list_all(&self) -> Result<Vec<PlaybackSession>, RepositoryError>;
    async fn remove(&self, id: &SessionId) -> Result<(), RepositoryError>;
}

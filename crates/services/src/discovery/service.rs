use std::collections::HashSet;

use domain::catalog::{Episode, EpisodeId, Movie, MovieId, Season, Series, TitleId, VersionId};
use domain::common::{Page, PageRequest};
use domain::discovery::{ContinueWatchingItem, Hub, HubItem, SearchKind, SearchResult};
use domain::error::{DiscoveryError, RepositoryError};
use domain::repository::{CatalogRepository, ProgressRepository, SearchIndex};
use domain::service::DiscoveryService;
use domain::session::{NowPlaying, PlaybackSession};
use domain::user::UserId;

use crate::discovery::{
    continue_watching, home_hubs, next_episodes, next_movies, recently_added, resume_card,
};

const ALL: PageRequest = PageRequest {
    offset: 0,
    limit: u32::MAX,
};
const HUB_LIMIT: usize = 20;

#[derive(Clone)]
pub struct DiscoveryServiceImpl<C, Se, Pr> {
    catalog: C,
    search_index: Se,
    progress: Pr,
}

impl<C, Se, Pr> DiscoveryServiceImpl<C, Se, Pr> {
    pub fn new(catalog: C, search_index: Se, progress: Pr) -> Self {
        Self {
            catalog,
            search_index,
            progress,
        }
    }
}

impl<C, Se, Pr> DiscoveryServiceImpl<C, Se, Pr>
where
    C: CatalogRepository + Sync,
    Se: SearchIndex + Sync,
    Pr: ProgressRepository + Sync,
{
    async fn all_movies(&self) -> Result<Vec<Movie>, RepositoryError> {
        Ok(self.catalog.list_movies(ALL).await?.items)
    }

    async fn all_series(&self) -> Result<Vec<Series>, RepositoryError> {
        Ok(self.catalog.list_series(ALL).await?.items)
    }

    async fn all_seasons(&self) -> Result<Vec<Season>, RepositoryError> {
        let mut seasons = Vec::new();
        for series in self.all_series().await? {
            seasons.extend(self.catalog.list_seasons(&series.id).await?);
        }
        Ok(seasons)
    }

    async fn all_episodes(&self) -> Result<Vec<Episode>, RepositoryError> {
        let mut episodes = Vec::new();
        for season in self.all_seasons().await? {
            episodes.extend(self.catalog.list_episodes(&season.id).await?);
        }
        Ok(episodes)
    }

    async fn completed_versions(
        &self,
        user: &UserId,
    ) -> Result<HashSet<VersionId>, RepositoryError> {
        let mut completed = HashSet::new();
        for history in self.progress.history(user, ALL).await?.items {
            if history.completed {
                for version in self.catalog.list_versions(&history.title, ALL).await?.items {
                    completed.insert(version.id);
                }
            }
        }
        Ok(completed)
    }

    async fn watched_episodes(&self, user: &UserId) -> Result<HashSet<EpisodeId>, RepositoryError> {
        Ok(self
            .progress
            .history(user, ALL)
            .await?
            .items
            .into_iter()
            .filter(|history| history.watched)
            .filter_map(|history| match history.title {
                TitleId::Episode(id) => Some(id),
                TitleId::Movie(_) => None,
            })
            .collect())
    }

    async fn watched_movies(&self, user: &UserId) -> Result<HashSet<MovieId>, RepositoryError> {
        Ok(self
            .progress
            .history(user, ALL)
            .await?
            .items
            .into_iter()
            .filter(|history| history.watched)
            .filter_map(|history| match history.title {
                TitleId::Movie(id) => Some(id),
                TitleId::Episode(_) => None,
            })
            .collect())
    }

    async fn title_hub_item(&self, title: &TitleId) -> Result<Option<HubItem>, RepositoryError> {
        match title {
            TitleId::Movie(id) => Ok(self.catalog.get_movie(id).await?.map(HubItem::Movie)),
            TitleId::Episode(_) => Ok(None),
        }
    }
}

impl<C, Se, Pr> DiscoveryService for DiscoveryServiceImpl<C, Se, Pr>
where
    C: CatalogRepository + Sync,
    Se: SearchIndex + Sync,
    Pr: ProgressRepository + Sync,
{
    async fn search(
        &self,
        _user: &UserId,
        query: &str,
        types: &[SearchKind],
        page: PageRequest,
    ) -> Result<Page<SearchResult>, DiscoveryError> {
        Ok(self.search_index.search(query, types, page).await?)
    }

    async fn continue_watching(
        &self,
        user: &UserId,
    ) -> Result<Vec<ContinueWatchingItem>, DiscoveryError> {
        let progress = self.progress.list_in_progress(user).await?;
        let completed = self.completed_versions(user).await?;
        let mut items = Vec::new();
        for entry in continue_watching(&progress, &completed) {
            if let Some(card) =
                resume_card(&self.catalog, &entry.version, entry.position_ms).await?
            {
                items.push(ContinueWatchingItem {
                    progress: entry,
                    card,
                });
            }
        }
        Ok(items)
    }

    async fn now_playing(
        &self,
        sessions: Vec<PlaybackSession>,
    ) -> Result<Vec<NowPlaying>, DiscoveryError> {
        let mut items = Vec::with_capacity(sessions.len());
        for session in sessions {
            if let Some(card) =
                resume_card(&self.catalog, &session.version, session.position_ms).await?
            {
                items.push(NowPlaying { session, card });
            }
        }
        Ok(items)
    }

    async fn next_episodes(&self, user: &UserId) -> Result<Vec<Episode>, DiscoveryError> {
        let seasons = self.all_seasons().await?;
        let episodes = self.all_episodes().await?;
        let watched = self.watched_episodes(user).await?;
        Ok(next_episodes(&seasons, &episodes, &watched))
    }

    async fn next_movies(&self, user: &UserId) -> Result<Vec<Movie>, DiscoveryError> {
        let collections = self.catalog.list_collections(ALL).await?.items;
        let movies = self.all_movies().await?;
        let watched = self.watched_movies(user).await?;
        Ok(next_movies(&collections, &movies, &watched))
    }

    async fn home_hubs(&self, user: &UserId) -> Result<Vec<Hub>, DiscoveryError> {
        let recently = recently_added(
            &self.all_movies().await?,
            &self.all_series().await?,
            HUB_LIMIT,
        );
        let on_deck: Vec<HubItem> = self
            .next_movies(user)
            .await?
            .into_iter()
            .map(HubItem::Movie)
            .collect();
        let mut continue_row = Vec::new();
        for item in self.continue_watching(user).await? {
            if let Some(hub) = self.title_hub_item(&item.card.title).await? {
                continue_row.push(hub);
            }
        }
        Ok(home_hubs(recently, on_deck, continue_row))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{MockCatalogRepo, MockProgressRepo, MockSearchIndex};
    use domain::catalog::{Collection, CollectionId, MovieId, SeasonId, SeriesId, Version};
    use domain::catalog::{EpisodeId, VersionId};
    use domain::common::Quality;
    use domain::library::LibraryId;
    use domain::playback::{PlaybackProgress, WatchHistory};
    use domain::session::{
        DeliveryMode, PlaybackSession, PlaybackState, SelectedTracks, SessionId,
    };
    use jiff::{SignedDuration, Timestamp};

    type Svc = DiscoveryServiceImpl<MockCatalogRepo, MockSearchIndex, MockProgressRepo>;

    fn user() -> UserId {
        UserId("u1".into())
    }

    fn page() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 10,
        }
    }

    fn movie(id: &str, seconds: i64) -> Movie {
        Movie {
            id: MovieId(id.into()),
            title: id.into(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            added_at: Timestamp::UNIX_EPOCH + SignedDuration::from_secs(seconds),
            updated_at: Timestamp::UNIX_EPOCH + SignedDuration::from_secs(seconds),
            artwork: Vec::new(),
        }
    }

    fn series(id: &str) -> Series {
        Series {
            id: SeriesId(id.into()),
            title: id.into(),
            year: None,
            overview: None,
            content_rating: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    fn season(id: &str, series: &str) -> Season {
        Season {
            id: SeasonId(id.into()),
            series: SeriesId(series.into()),
            number: 1,
            title: None,
            overview: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    fn episode(id: &str, season: &str, number: u16) -> Episode {
        Episode {
            id: EpisodeId(id.into()),
            season: SeasonId(season.into()),
            number,
            title: id.into(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    fn version(id: &str, title: TitleId) -> Version {
        Version {
            id: VersionId(id.into()),
            title,
            library: LibraryId("lib1".into()),
            quality: Quality::Hd,
            container: "mkv".into(),
            path: format!("/media/{id}.mkv"),
            size_bytes: 1,
            duration_ms: 1000,
            edition: None,
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn history(title: TitleId, watched: bool, completed: bool) -> WatchHistory {
        WatchHistory {
            user: user(),
            title,
            watched,
            play_count: 1,
            last_watched_at: None,
            completed,
        }
    }

    async fn seeded() -> Svc {
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(movie("m1", 10));
        catalog.add_movie(movie("m2", 20));
        catalog.add_series(series("s1"));
        catalog.add_season(season("se1", "s1"));
        catalog.add_episode(episode("e1", "se1", 1));
        catalog.add_episode(episode("e2", "se1", 2));
        catalog.add_collection(Collection {
            id: CollectionId("c1".into()),
            name: "Saga".into(),
            overview: None,
            movies: vec![MovieId("m1".into()), MovieId("m2".into())],
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog.add_version(version("v1", TitleId::Movie(MovieId("m1".into()))));
        catalog.add_version(version("ev1", TitleId::Episode(EpisodeId("e1".into()))));

        let search = MockSearchIndex::new();
        search.add(SearchResult::Movie(movie("m1", 10)));

        let progress = MockProgressRepo::new();
        progress
            .upsert(PlaybackProgress {
                user: user(),
                version: VersionId("v1".into()),
                position_ms: 1234,
                updated_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .unwrap();
        progress
            .record_history(history(TitleId::Movie(MovieId("m1".into())), true, false))
            .await
            .unwrap();
        progress
            .record_history(history(
                TitleId::Episode(EpisodeId("e1".into())),
                true,
                false,
            ))
            .await
            .unwrap();

        DiscoveryServiceImpl::new(catalog, search, progress)
    }

    #[tokio::test]
    async fn search_delegates_to_index() {
        let svc = seeded().await;
        let hits = svc.search(&user(), "m1", &[], page()).await.unwrap();
        assert_eq!(hits.total, 1);
    }

    fn session(id: &str, version: &str, position_ms: u64) -> PlaybackSession {
        PlaybackSession {
            id: SessionId(id.into()),
            user: user(),
            device: None,
            version: VersionId(version.into()),
            mode: DeliveryMode::Direct,
            position_ms,
            state: PlaybackState::Playing,
            selected: SelectedTracks {
                audio_track: None,
                subtitle_track: None,
                subtitle_delivery: None,
            },
            started_at: Timestamp::UNIX_EPOCH,
            last_heartbeat_at: Timestamp::UNIX_EPOCH,
        }
    }

    #[tokio::test]
    async fn continue_watching_enriches_with_card() {
        let svc = seeded().await;
        let items = svc.continue_watching(&user()).await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].progress.version, VersionId("v1".into()));
        assert_eq!(items[0].card.display_title, "m1");
        assert_eq!(items[0].card.duration_ms, 1000);
        assert_eq!(items[0].card.progress_percent, 100);
    }

    #[tokio::test]
    async fn now_playing_enriches_and_skips_dangling_versions() {
        let svc = seeded().await;
        let now = svc
            .now_playing(vec![session("s1", "v1", 500), session("s2", "ghost", 10)])
            .await
            .unwrap();
        assert_eq!(now.len(), 1);
        assert_eq!(now[0].session.id, SessionId("s1".into()));
        assert_eq!(now[0].card.display_title, "m1");
        assert_eq!(now[0].card.duration_ms, 1000);
        assert_eq!(now[0].card.progress_percent, 50);
    }

    #[tokio::test]
    async fn continue_watching_excludes_completed() {
        let svc = seeded().await;
        let items = svc.continue_watching(&user()).await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].progress.version, VersionId("v1".into()));

        let completed = MockProgressRepo::new();
        completed
            .upsert(PlaybackProgress {
                user: user(),
                version: VersionId("v1".into()),
                position_ms: 1234,
                updated_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .unwrap();
        completed
            .record_history(history(TitleId::Movie(MovieId("m1".into())), true, true))
            .await
            .unwrap();
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version("v1", TitleId::Movie(MovieId("m1".into()))));
        let svc = DiscoveryServiceImpl::new(catalog, MockSearchIndex::new(), completed);
        assert!(svc.continue_watching(&user()).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn next_episodes_and_movies() {
        let svc = seeded().await;
        let next_eps = svc.next_episodes(&user()).await.unwrap();
        assert_eq!(next_eps.len(), 1);
        assert_eq!(next_eps[0].id, EpisodeId("e2".into()));

        let next_films = svc.next_movies(&user()).await.unwrap();
        assert_eq!(next_films.len(), 1);
        assert_eq!(next_films[0].id, MovieId("m2".into()));
    }

    #[tokio::test]
    async fn home_hubs_composes_rows() {
        let svc = seeded().await;
        let hubs = svc.home_hubs(&user()).await.unwrap();
        let ids: Vec<&str> = hubs.iter().map(|h| h.id.as_str()).collect();
        assert!(ids.contains(&"recently_added"));
        assert!(ids.contains(&"continue_watching"));
    }

    #[tokio::test]
    async fn home_hubs_continue_row_keeps_movies_and_skips_the_rest() {
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(movie("mm", 0));
        catalog.add_version(version("vm", TitleId::Movie(MovieId("mm".into()))));
        catalog.add_version(version("vmiss", TitleId::Movie(MovieId("gone".into()))));
        catalog.add_version(version("vep", TitleId::Episode(EpisodeId("e1".into()))));

        let progress = MockProgressRepo::new();
        for id in ["vm", "vmiss", "vep", "vnone"] {
            progress
                .upsert(PlaybackProgress {
                    user: user(),
                    version: VersionId(id.into()),
                    position_ms: 100,
                    updated_at: Timestamp::UNIX_EPOCH,
                })
                .await
                .unwrap();
        }

        let svc = DiscoveryServiceImpl::new(catalog, MockSearchIndex::new(), progress);
        let hubs = svc.home_hubs(&user()).await.unwrap();
        let continue_row = hubs
            .iter()
            .find(|hub| hub.id == "continue_watching")
            .expect("continue_watching row present");
        assert_eq!(continue_row.items.len(), 1);
        assert!(matches!(
            &continue_row.items[0],
            HubItem::Movie(movie) if movie.id == MovieId("mm".into())
        ));
    }

    #[tokio::test]
    async fn backend_errors_propagate() {
        let catalog = MockCatalogRepo::new();
        catalog.set_fail();
        let progress = MockProgressRepo::new();
        progress.set_fail();
        let search = MockSearchIndex::new();
        search.set_fail();
        let svc = DiscoveryServiceImpl::new(catalog, search, progress);
        assert!(svc.search(&user(), "x", &[], page()).await.is_err());
        assert!(svc.continue_watching(&user()).await.is_err());
        assert!(svc.now_playing(vec![session("s1", "v1", 1)]).await.is_err());
        assert!(svc.next_episodes(&user()).await.is_err());
        assert!(svc.next_movies(&user()).await.is_err());
        assert!(svc.home_hubs(&user()).await.is_err());
    }
}

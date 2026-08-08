use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use domain::catalog::{TitleId, VersionId};
use domain::common::{Page, PageRequest};
use domain::error::UserError;
use domain::playback::{
    Favorite, PlaybackProgress, TitleState, WatchHistory, WatchTarget, WatchedRollup, WatchlistItem,
};
use domain::repository::{CatalogRepository, PreferencesRepository, ProgressRepository};
use domain::service::UserLibraryService;
use domain::user::UserId;
use jiff::Timestamp;

pub struct UserLibraryServiceImpl<Pr, Pf, C> {
    progress: Arc<Pr>,
    preferences: Arc<Pf>,
    catalog: Arc<C>,
}

impl<Pr, Pf, C> UserLibraryServiceImpl<Pr, Pf, C> {
    pub fn new(progress: Arc<Pr>, preferences: Arc<Pf>, catalog: Arc<C>) -> Self {
        Self {
            progress,
            preferences,
            catalog,
        }
    }
}

impl<Pr, Pf, C> Clone for UserLibraryServiceImpl<Pr, Pf, C> {
    fn clone(&self) -> Self {
        Self {
            progress: Arc::clone(&self.progress),
            preferences: Arc::clone(&self.preferences),
            catalog: Arc::clone(&self.catalog),
        }
    }
}

impl<Pr, Pf, C> UserLibraryServiceImpl<Pr, Pf, C>
where
    C: CatalogRepository + Send + Sync,
{
    async fn leaf_titles(&self, target: &WatchTarget) -> Result<Vec<TitleId>, UserError> {
        Ok(match target {
            WatchTarget::Movie(id) => vec![TitleId::Movie(id.clone())],
            WatchTarget::Episode(id) => vec![TitleId::Episode(id.clone())],
            WatchTarget::Season(id) => self
                .catalog
                .list_episodes(id)
                .await?
                .into_iter()
                .map(|episode| TitleId::Episode(episode.id))
                .collect(),
            WatchTarget::Series(id) => {
                let mut titles = Vec::new();
                for season in self.catalog.list_seasons(id).await? {
                    for episode in self.catalog.list_episodes(&season.id).await? {
                        titles.push(TitleId::Episode(episode.id));
                    }
                }
                titles
            }
        })
    }
}

impl<Pr, Pf, C> UserLibraryService for UserLibraryServiceImpl<Pr, Pf, C>
where
    Pr: ProgressRepository + Send + Sync,
    Pf: PreferencesRepository + Send + Sync,
    C: CatalogRepository + Send + Sync,
{
    async fn watchlist(&self, user: &UserId) -> Result<Vec<WatchlistItem>, UserError> {
        Ok(self.preferences.list_watchlist(user).await?)
    }

    async fn add_to_watchlist(&self, user: &UserId, title: &TitleId) -> Result<(), UserError> {
        self.preferences
            .add_watchlist(WatchlistItem {
                user: user.clone(),
                title: title.clone(),
                added_at: Timestamp::now(),
            })
            .await?;
        Ok(())
    }

    async fn remove_from_watchlist(&self, user: &UserId, title_id: &str) -> Result<(), UserError> {
        self.preferences.remove_watchlist(user, title_id).await?;
        Ok(())
    }

    async fn favorites(&self, user: &UserId) -> Result<Vec<Favorite>, UserError> {
        Ok(self.preferences.list_favorites(user).await?)
    }

    async fn add_favorite(&self, user: &UserId, title: &TitleId) -> Result<(), UserError> {
        self.preferences
            .add_favorite(Favorite {
                user: user.clone(),
                title: title.clone(),
                added_at: Timestamp::now(),
            })
            .await?;
        Ok(())
    }

    async fn remove_favorite(&self, user: &UserId, title_id: &str) -> Result<(), UserError> {
        self.preferences.remove_favorite(user, title_id).await?;
        Ok(())
    }

    async fn history(
        &self,
        user: &UserId,
        page: PageRequest,
    ) -> Result<Page<WatchHistory>, UserError> {
        Ok(self.progress.history(user, page).await?)
    }

    async fn progress(
        &self,
        user: &UserId,
        version: &VersionId,
    ) -> Result<Option<PlaybackProgress>, UserError> {
        Ok(self.progress.get(user, version).await?)
    }

    async fn clear_progress(&self, user: &UserId, version: &VersionId) -> Result<(), UserError> {
        self.progress.delete(user, version).await?;
        Ok(())
    }

    async fn set_watched(
        &self,
        user: &UserId,
        target: &WatchTarget,
        watched: bool,
    ) -> Result<(), UserError> {
        let last_watched_at = watched.then(Timestamp::now);
        let make = |title: TitleId| WatchHistory {
            user: user.clone(),
            title,
            watched,
            play_count: u32::from(watched),
            last_watched_at,
            completed: watched,
        };
        for title in self.leaf_titles(target).await? {
            self.progress.record_history(make(title.clone())).await?;
            if !watched {
                for version in self
                    .catalog
                    .list_versions(&title, PageRequest::ALL)
                    .await?
                    .items
                {
                    self.progress.delete(user, &version.id).await?;
                }
            }
        }
        Ok(())
    }

    async fn title_states(
        &self,
        user: &UserId,
        titles: &[TitleId],
    ) -> Result<Vec<TitleState>, UserError> {
        let favorites: HashSet<TitleId> = self
            .preferences
            .list_favorites(user)
            .await?
            .into_iter()
            .map(|f| f.title)
            .collect();
        let watchlisted: HashSet<TitleId> = self
            .preferences
            .list_watchlist(user)
            .await?
            .into_iter()
            .map(|w| w.title)
            .collect();
        let history: HashMap<TitleId, (bool, bool)> = self
            .progress
            .history(user, PageRequest::ALL)
            .await?
            .items
            .into_iter()
            .map(|h| (h.title, (h.watched, h.completed)))
            .collect();
        Ok(titles
            .iter()
            .map(|title| {
                let (watched, completed) = history.get(title).copied().unwrap_or((false, false));
                TitleState {
                    title: title.clone(),
                    favorite: favorites.contains(title),
                    watchlisted: watchlisted.contains(title),
                    watched,
                    completed,
                }
            })
            .collect())
    }

    async fn watched_rollups(
        &self,
        user: &UserId,
        targets: &[WatchTarget],
    ) -> Result<Vec<WatchedRollup>, UserError> {
        let history: HashMap<TitleId, (bool, bool)> = self
            .progress
            .history(user, PageRequest::ALL)
            .await?
            .items
            .into_iter()
            .map(|h| (h.title, (h.watched, h.completed)))
            .collect();
        let mut rollups = Vec::with_capacity(targets.len());
        for target in targets {
            let leaves = self.leaf_titles(target).await?;
            let total = leaves.len() as u32;
            let mut watched_episodes = 0;
            let mut all_completed = true;
            for title in &leaves {
                let (watched, completed) = history.get(title).copied().unwrap_or((false, false));
                if watched {
                    watched_episodes += 1;
                }
                all_completed &= completed;
            }
            rollups.push(WatchedRollup {
                target: target.clone(),
                watched: total > 0 && watched_episodes == total,
                completed: total > 0 && all_completed,
                watched_episodes,
                total_episodes: total,
            });
        }
        Ok(rollups)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{MockCatalogRepo, MockPreferencesRepo, MockProgressRepo};
    use domain::catalog::{
        Episode, EpisodeId, MovieId, Season, SeasonId, Series, SeriesId, Version,
    };
    use domain::common::Quality;
    use domain::library::LibraryId;
    use std::collections::HashSet;

    type Svc = UserLibraryServiceImpl<MockProgressRepo, MockPreferencesRepo, MockCatalogRepo>;

    fn service() -> Svc {
        UserLibraryServiceImpl::new(
            Arc::new(MockProgressRepo::new()),
            Arc::new(MockPreferencesRepo::new()),
            Arc::new(MockCatalogRepo::new()),
        )
    }

    fn user() -> UserId {
        UserId("u1".into())
    }

    fn title() -> TitleId {
        TitleId::Movie(MovieId("m1".into()))
    }

    fn version(id: &str) -> Version {
        Version {
            id: VersionId(id.into()),
            title: TitleId::Movie(MovieId("m1".into())),
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

    fn page() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 100,
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

    fn episode(id: &str, season: &str) -> Episode {
        Episode {
            id: EpisodeId(id.into()),
            season: SeasonId(season.into()),
            number: 1,
            title: id.into(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    async fn watched_ids(svc: &Svc) -> HashSet<String> {
        svc.history(&user(), page())
            .await
            .unwrap()
            .items
            .into_iter()
            .map(|h| h.title.id().to_owned())
            .collect()
    }

    #[tokio::test]
    async fn watchlist_and_favorites_roundtrip() {
        let svc = service();
        assert!(svc.watchlist(&user()).await.unwrap().is_empty());
        svc.add_to_watchlist(&user(), &title()).await.unwrap();
        assert_eq!(svc.watchlist(&user()).await.unwrap().len(), 1);
        svc.remove_from_watchlist(&user(), "m1").await.unwrap();
        assert!(svc.watchlist(&user()).await.unwrap().is_empty());

        svc.add_favorite(&user(), &title()).await.unwrap();
        assert_eq!(svc.favorites(&user()).await.unwrap().len(), 1);
        svc.remove_favorite(&user(), "m1").await.unwrap();
        assert!(svc.favorites(&user()).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn history_and_progress() {
        let progress = Arc::new(MockProgressRepo::new());
        let svc = UserLibraryServiceImpl::new(
            Arc::clone(&progress),
            Arc::new(MockPreferencesRepo::new()),
            Arc::new(MockCatalogRepo::new()),
        );
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
            .record_history(WatchHistory {
                user: user(),
                title: title(),
                watched: true,
                play_count: 1,
                last_watched_at: None,
                completed: false,
            })
            .await
            .unwrap();

        assert_eq!(svc.history(&user(), page()).await.unwrap().total, 1);
        assert_eq!(
            svc.progress(&user(), &VersionId("v1".into()))
                .await
                .unwrap()
                .unwrap()
                .position_ms,
            1234
        );

        svc.clear_progress(&user(), &VersionId("v1".into()))
            .await
            .unwrap();
        assert!(
            svc.progress(&user(), &VersionId("v1".into()))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn unwatch_resets_progress_for_the_titles_versions() {
        let progress = Arc::new(MockProgressRepo::new());
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version("v1"));
        let svc = UserLibraryServiceImpl::new(
            Arc::clone(&progress),
            Arc::new(MockPreferencesRepo::new()),
            Arc::new(catalog),
        );
        progress
            .upsert(PlaybackProgress {
                user: user(),
                version: VersionId("v1".into()),
                position_ms: 1234,
                updated_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .unwrap();

        svc.set_watched(&user(), &WatchTarget::Movie(MovieId("m1".into())), true)
            .await
            .unwrap();
        assert!(
            svc.progress(&user(), &VersionId("v1".into()))
                .await
                .unwrap()
                .is_some()
        );

        svc.set_watched(&user(), &WatchTarget::Movie(MovieId("m1".into())), false)
            .await
            .unwrap();
        assert!(
            svc.progress(&user(), &VersionId("v1".into()))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn cloneable_shares_state() {
        let svc = service();
        let clone = svc.clone();
        svc.add_favorite(&user(), &title()).await.unwrap();
        assert_eq!(clone.favorites(&user()).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn backend_errors_propagate() {
        let preferences = MockPreferencesRepo::new();
        preferences.set_fail();
        let progress = MockProgressRepo::new();
        progress.set_fail();
        let catalog = MockCatalogRepo::new();
        catalog.set_fail();
        let svc = UserLibraryServiceImpl::new(
            Arc::new(progress),
            Arc::new(preferences),
            Arc::new(catalog),
        );

        assert!(svc.watchlist(&user()).await.is_err());
        assert!(svc.add_to_watchlist(&user(), &title()).await.is_err());
        assert!(svc.remove_from_watchlist(&user(), "m1").await.is_err());
        assert!(svc.favorites(&user()).await.is_err());
        assert!(svc.add_favorite(&user(), &title()).await.is_err());
        assert!(svc.remove_favorite(&user(), "m1").await.is_err());
        assert!(svc.history(&user(), page()).await.is_err());
        assert!(
            svc.progress(&user(), &VersionId("v1".into()))
                .await
                .is_err()
        );
        assert!(
            svc.clear_progress(&user(), &VersionId("v1".into()))
                .await
                .is_err()
        );
        assert!(
            svc.set_watched(&user(), &WatchTarget::Movie(MovieId("m1".into())), true)
                .await
                .is_err()
        );
        assert!(
            svc.set_watched(&user(), &WatchTarget::Season(SeasonId("se1".into())), true)
                .await
                .is_err()
        );
        assert!(
            svc.set_watched(&user(), &WatchTarget::Series(SeriesId("sr1".into())), true)
                .await
                .is_err()
        );
        assert!(svc.title_states(&user(), &[title()]).await.is_err());
        assert!(
            svc.watched_rollups(&user(), &[WatchTarget::Movie(MovieId("m1".into()))])
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn title_states_reflects_favorites_watchlist_and_history() {
        let svc = service();
        let m1 = TitleId::Movie(MovieId("m1".into()));
        let m2 = TitleId::Movie(MovieId("m2".into()));
        let m3 = TitleId::Movie(MovieId("m3".into()));
        let m4 = TitleId::Movie(MovieId("m4".into()));
        svc.add_favorite(&user(), &m1).await.unwrap();
        svc.add_to_watchlist(&user(), &m2).await.unwrap();
        svc.set_watched(&user(), &WatchTarget::Movie(MovieId("m3".into())), true)
            .await
            .unwrap();

        let states = svc
            .title_states(&user(), &[m3.clone(), m2.clone(), m1.clone(), m4.clone()])
            .await
            .unwrap();

        assert_eq!(
            states,
            vec![
                TitleState {
                    title: m3,
                    favorite: false,
                    watchlisted: false,
                    watched: true,
                    completed: true,
                },
                TitleState {
                    title: m2,
                    favorite: false,
                    watchlisted: true,
                    watched: false,
                    completed: false,
                },
                TitleState {
                    title: m1,
                    favorite: true,
                    watchlisted: false,
                    watched: false,
                    completed: false,
                },
                TitleState {
                    title: m4,
                    favorite: false,
                    watchlisted: false,
                    watched: false,
                    completed: false,
                },
            ]
        );
    }

    #[tokio::test]
    async fn title_states_empty_list_is_empty() {
        let svc = service();
        assert!(svc.title_states(&user(), &[]).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn set_watched_fans_out_to_episodes() {
        let progress = Arc::new(MockProgressRepo::new());
        let catalog = Arc::new(MockCatalogRepo::new());
        catalog.add_series(series("sr1"));
        catalog.add_season(season("se1", "sr1"));
        catalog.add_season(season("se2", "sr1"));
        catalog.add_episode(episode("e1", "se1"));
        catalog.add_episode(episode("e2", "se1"));
        catalog.add_episode(episode("e3", "se2"));
        let svc = UserLibraryServiceImpl::new(
            Arc::clone(&progress),
            Arc::new(MockPreferencesRepo::new()),
            Arc::clone(&catalog),
        );

        svc.set_watched(&user(), &WatchTarget::Movie(MovieId("m1".into())), true)
            .await
            .unwrap();
        svc.set_watched(&user(), &WatchTarget::Episode(EpisodeId("e9".into())), true)
            .await
            .unwrap();
        svc.set_watched(&user(), &WatchTarget::Season(SeasonId("se1".into())), true)
            .await
            .unwrap();
        svc.set_watched(&user(), &WatchTarget::Series(SeriesId("sr1".into())), true)
            .await
            .unwrap();

        assert_eq!(
            watched_ids(&svc).await,
            HashSet::from(["m1", "e9", "e1", "e2", "e3"].map(String::from))
        );
        let history = svc.history(&user(), page()).await.unwrap();
        assert!(history.items.iter().all(|h| h.completed && h.watched));
        assert!(history.items.iter().all(|h| h.play_count == 1));
        assert!(history.items.iter().all(|h| h.last_watched_at.is_some()));
    }

    #[tokio::test]
    async fn set_unwatched_clears_flags() {
        let svc = service();
        svc.set_watched(&user(), &WatchTarget::Movie(MovieId("m1".into())), true)
            .await
            .unwrap();
        svc.set_watched(&user(), &WatchTarget::Movie(MovieId("m1".into())), false)
            .await
            .unwrap();
        let history = svc.history(&user(), page()).await.unwrap();
        assert_eq!(history.total, 1);
        let entry = &history.items[0];
        assert!(!entry.watched);
        assert!(!entry.completed);
        assert_eq!(entry.play_count, 0);
        assert!(entry.last_watched_at.is_none());
    }

    #[tokio::test]
    async fn set_watched_empty_season_and_series_are_noops() {
        let svc = service();
        svc.set_watched(&user(), &WatchTarget::Season(SeasonId("se1".into())), true)
            .await
            .unwrap();
        svc.set_watched(&user(), &WatchTarget::Series(SeriesId("sr1".into())), true)
            .await
            .unwrap();
        assert!(svc.history(&user(), page()).await.unwrap().items.is_empty());
    }

    #[tokio::test]
    async fn watched_rollup_counts_partially_watched_season() {
        let catalog = Arc::new(MockCatalogRepo::new());
        catalog.add_series(series("sr1"));
        catalog.add_season(season("se1", "sr1"));
        catalog.add_episode(episode("e1", "se1"));
        catalog.add_episode(episode("e2", "se1"));
        catalog.add_episode(episode("e3", "se1"));
        let svc = UserLibraryServiceImpl::new(
            Arc::new(MockProgressRepo::new()),
            Arc::new(MockPreferencesRepo::new()),
            Arc::clone(&catalog),
        );
        svc.set_watched(&user(), &WatchTarget::Episode(EpisodeId("e1".into())), true)
            .await
            .unwrap();
        svc.set_watched(&user(), &WatchTarget::Episode(EpisodeId("e2".into())), true)
            .await
            .unwrap();

        let rollups = svc
            .watched_rollups(&user(), &[WatchTarget::Season(SeasonId("se1".into()))])
            .await
            .unwrap();
        assert_eq!(
            rollups,
            vec![WatchedRollup {
                target: WatchTarget::Season(SeasonId("se1".into())),
                watched: false,
                completed: false,
                watched_episodes: 2,
                total_episodes: 3,
            }]
        );
    }

    #[tokio::test]
    async fn watched_rollup_full_season_and_series_roll_up() {
        let catalog = Arc::new(MockCatalogRepo::new());
        catalog.add_series(series("sr1"));
        catalog.add_season(season("se1", "sr1"));
        catalog.add_season(season("se2", "sr1"));
        catalog.add_episode(episode("e1", "se1"));
        catalog.add_episode(episode("e2", "se1"));
        catalog.add_episode(episode("e3", "se2"));
        let svc = UserLibraryServiceImpl::new(
            Arc::new(MockProgressRepo::new()),
            Arc::new(MockPreferencesRepo::new()),
            Arc::clone(&catalog),
        );
        svc.set_watched(&user(), &WatchTarget::Series(SeriesId("sr1".into())), true)
            .await
            .unwrap();

        let rollups = svc
            .watched_rollups(
                &user(),
                &[
                    WatchTarget::Season(SeasonId("se1".into())),
                    WatchTarget::Series(SeriesId("sr1".into())),
                ],
            )
            .await
            .unwrap();
        assert_eq!(
            rollups,
            vec![
                WatchedRollup {
                    target: WatchTarget::Season(SeasonId("se1".into())),
                    watched: true,
                    completed: true,
                    watched_episodes: 2,
                    total_episodes: 2,
                },
                WatchedRollup {
                    target: WatchTarget::Series(SeriesId("sr1".into())),
                    watched: true,
                    completed: true,
                    watched_episodes: 3,
                    total_episodes: 3,
                },
            ]
        );
    }

    #[tokio::test]
    async fn watched_rollup_watched_without_completed() {
        let progress = Arc::new(MockProgressRepo::new());
        let catalog = Arc::new(MockCatalogRepo::new());
        catalog.add_series(series("sr1"));
        catalog.add_season(season("se1", "sr1"));
        catalog.add_episode(episode("e1", "se1"));
        catalog.add_episode(episode("e2", "se1"));
        for id in ["e1", "e2"] {
            progress
                .record_history(WatchHistory {
                    user: user(),
                    title: TitleId::Episode(EpisodeId(id.into())),
                    watched: true,
                    play_count: 1,
                    last_watched_at: None,
                    completed: false,
                })
                .await
                .unwrap();
        }
        let svc = UserLibraryServiceImpl::new(
            Arc::clone(&progress),
            Arc::new(MockPreferencesRepo::new()),
            Arc::clone(&catalog),
        );

        let rollups = svc
            .watched_rollups(&user(), &[WatchTarget::Season(SeasonId("se1".into()))])
            .await
            .unwrap();
        assert_eq!(
            rollups,
            vec![WatchedRollup {
                target: WatchTarget::Season(SeasonId("se1".into())),
                watched: true,
                completed: false,
                watched_episodes: 2,
                total_episodes: 2,
            }]
        );
    }

    #[tokio::test]
    async fn watched_rollup_handles_movie_and_episode_targets() {
        let catalog = Arc::new(MockCatalogRepo::new());
        catalog.add_series(series("sr1"));
        catalog.add_season(season("se1", "sr1"));
        catalog.add_episode(episode("e1", "se1"));
        let svc = UserLibraryServiceImpl::new(
            Arc::new(MockProgressRepo::new()),
            Arc::new(MockPreferencesRepo::new()),
            Arc::clone(&catalog),
        );
        svc.set_watched(&user(), &WatchTarget::Episode(EpisodeId("e1".into())), true)
            .await
            .unwrap();

        let rollups = svc
            .watched_rollups(
                &user(),
                &[
                    WatchTarget::Movie(MovieId("m1".into())),
                    WatchTarget::Episode(EpisodeId("e1".into())),
                ],
            )
            .await
            .unwrap();
        assert_eq!(
            rollups,
            vec![
                WatchedRollup {
                    target: WatchTarget::Movie(MovieId("m1".into())),
                    watched: false,
                    completed: false,
                    watched_episodes: 0,
                    total_episodes: 1,
                },
                WatchedRollup {
                    target: WatchTarget::Episode(EpisodeId("e1".into())),
                    watched: true,
                    completed: true,
                    watched_episodes: 1,
                    total_episodes: 1,
                },
            ]
        );
    }

    #[tokio::test]
    async fn watched_rollup_empty_container_is_false_and_zero() {
        let svc = service();
        let rollups = svc
            .watched_rollups(
                &user(),
                &[
                    WatchTarget::Season(SeasonId("se1".into())),
                    WatchTarget::Series(SeriesId("sr1".into())),
                ],
            )
            .await
            .unwrap();
        assert_eq!(
            rollups,
            vec![
                WatchedRollup {
                    target: WatchTarget::Season(SeasonId("se1".into())),
                    watched: false,
                    completed: false,
                    watched_episodes: 0,
                    total_episodes: 0,
                },
                WatchedRollup {
                    target: WatchTarget::Series(SeriesId("sr1".into())),
                    watched: false,
                    completed: false,
                    watched_episodes: 0,
                    total_episodes: 0,
                },
            ]
        );
    }

    #[tokio::test]
    async fn watched_rollup_empty_targets_is_empty() {
        let svc = service();
        assert!(svc.watched_rollups(&user(), &[]).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn watched_rollups_propagate_catalog_errors() {
        let catalog = MockCatalogRepo::new();
        catalog.set_fail();
        let svc = UserLibraryServiceImpl::new(
            Arc::new(MockProgressRepo::new()),
            Arc::new(MockPreferencesRepo::new()),
            Arc::new(catalog),
        );
        assert!(
            svc.watched_rollups(&user(), &[WatchTarget::Season(SeasonId("se1".into()))])
                .await
                .is_err()
        );
    }
}

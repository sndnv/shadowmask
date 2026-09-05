use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use domain::catalog::{EpisodeId, SeasonId, SeriesId, TitleId, VersionId};
use domain::common::{Page, PageRequest};
use domain::error::UserError;
use domain::playback::{
    Favorite, PlaybackProgress, TitleState, WatchHistory, WatchTarget, WatchedRollup,
    WatchlistItem, progress_percent,
};
use domain::repository::{CatalogRepository, PreferencesRepository, ProgressRepository};
use domain::service::UserLibraryService;
use domain::user::UserId;
use jiff::Timestamp;

use crate::discovery::continue_watching;

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
        Ok(self
            .leaves_for(std::slice::from_ref(target))
            .await?
            .pop()
            .unwrap_or_default())
    }

    async fn leaves_for(&self, targets: &[WatchTarget]) -> Result<Vec<Vec<TitleId>>, UserError> {
        let series: Vec<SeriesId> = targets
            .iter()
            .filter_map(|target| match target {
                WatchTarget::Series(id) => Some(id.clone()),
                _ => None,
            })
            .collect();
        let seasons: Vec<SeasonId> = targets
            .iter()
            .filter_map(|target| match target {
                WatchTarget::Season(id) => Some(id.clone()),
                _ => None,
            })
            .collect();
        let by_series = self.catalog.episode_ids_for_series(&series).await?;
        let by_season = self.catalog.episode_ids_for_seasons(&seasons).await?;
        let episodes = |found: Option<&Vec<EpisodeId>>| {
            found
                .map(|ids| ids.iter().cloned().map(TitleId::Episode).collect())
                .unwrap_or_default()
        };
        Ok(targets
            .iter()
            .map(|target| match target {
                WatchTarget::Movie(id) => vec![TitleId::Movie(id.clone())],
                WatchTarget::Episode(id) => vec![TitleId::Episode(id.clone())],
                WatchTarget::Season(id) => episodes(by_season.get(id)),
                WatchTarget::Series(id) => episodes(by_series.get(id)),
            })
            .collect())
    }
}

impl<Pr, Pf, C> UserLibraryServiceImpl<Pr, Pf, C>
where
    Pr: ProgressRepository + Send + Sync,
    C: CatalogRepository + Send + Sync,
{
    async fn resume_progress(&self, user: &UserId) -> Result<HashMap<TitleId, u8>, UserError> {
        let started = self.progress.list_in_progress(user).await?;
        let mut percent = HashMap::new();
        for entry in continue_watching(&started) {
            let Some(version) = self.catalog.get_version(&entry.version).await? else {
                continue;
            };
            percent
                .entry(version.title)
                .or_insert_with(|| progress_percent(entry.position_ms, version.duration_ms));
        }
        Ok(percent)
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

    async fn remove_from_history(&self, user: &UserId, title_id: &str) -> Result<(), UserError> {
        self.progress.delete_history(user, title_id).await?;
        Ok(())
    }

    async fn clear_history(&self, user: &UserId) -> Result<(), UserError> {
        self.progress.clear_history(user).await?;
        Ok(())
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
        for title in self.leaf_titles(target).await? {
            self.progress
                .set_watched_flags(user, &title, watched)
                .await?;
            if watched {
                self.preferences.remove_watchlist(user, title.id()).await?;
            }
            for version in self
                .catalog
                .list_versions(&title, PageRequest::ALL)
                .await?
                .items
            {
                self.progress.delete(user, &version.id).await?;
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
            .watched_state(user)
            .await?
            .into_iter()
            .map(|h| (h.title, (h.watched, h.completed)))
            .collect();
        let resumable = self.resume_progress(user).await?;
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
                    progress_percent: resumable.get(title).copied().unwrap_or(0),
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
            .watched_state(user)
            .await?
            .into_iter()
            .map(|h| (h.title, (h.watched, h.completed)))
            .collect();
        let mut rollups = Vec::with_capacity(targets.len());
        for (target, leaves) in targets.iter().zip(self.leaves_for(targets).await?) {
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
    use domain::catalog::{
        Episode, EpisodeId, MovieId, Season, SeasonId, Series, SeriesId, Version,
    };
    use domain::common::Quality;
    use domain::library::LibraryId;
    use jiff::SignedDuration;
    use mocks::{MockCatalogRepo, MockPreferencesRepo, MockProgressRepo};
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
            sort_title: id.into(),
            year: None,
            overview: None,
            content_rating: None,
            manually_edited: false,
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
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    async fn watched_ids(progress: &MockProgressRepo) -> HashSet<String> {
        progress
            .watched_state(&user())
            .await
            .unwrap()
            .into_iter()
            .filter(|h| h.watched)
            .map(|h| h.title.id().to_owned())
            .collect()
    }

    async fn saved_ids(svc: &Svc) -> HashSet<String> {
        svc.watchlist(&user())
            .await
            .unwrap()
            .into_iter()
            .map(|item| item.title.id().to_owned())
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
                audio_track: None,
                subtitle: None,
                updated_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .unwrap();
        progress
            .record_view(&user(), &title(), Timestamp::UNIX_EPOCH)
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
    async fn marking_watched_resets_progress_for_the_titles_versions() {
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
                audio_track: None,
                subtitle: None,
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
                .is_none()
        );

        progress
            .upsert(PlaybackProgress {
                user: user(),
                version: VersionId("v1".into()),
                position_ms: 1234,
                audio_track: None,
                subtitle: None,
                updated_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .unwrap();
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
                    progress_percent: 0,
                },
                TitleState {
                    title: m2,
                    favorite: false,
                    watchlisted: true,
                    watched: false,
                    completed: false,
                    progress_percent: 0,
                },
                TitleState {
                    title: m1,
                    favorite: true,
                    watchlisted: false,
                    watched: false,
                    completed: false,
                    progress_percent: 0,
                },
                TitleState {
                    title: m4,
                    favorite: false,
                    watchlisted: false,
                    watched: false,
                    completed: false,
                    progress_percent: 0,
                },
            ]
        );
    }

    #[tokio::test]
    async fn title_states_carries_resume_progress_from_the_most_recent_version() {
        let progress = Arc::new(MockProgressRepo::new());
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version("v1"));
        catalog.add_version(version("v2"));
        let svc = UserLibraryServiceImpl::new(
            Arc::clone(&progress),
            Arc::new(MockPreferencesRepo::new()),
            Arc::new(catalog),
        );
        for (id, position_ms, seconds) in [("v1", 250, 10), ("v2", 700, 20)] {
            progress
                .upsert(PlaybackProgress {
                    user: user(),
                    version: VersionId(id.into()),
                    position_ms,
                    audio_track: None,
                    subtitle: None,
                    updated_at: Timestamp::UNIX_EPOCH + SignedDuration::from_secs(seconds),
                })
                .await
                .unwrap();
        }

        let m1 = TitleId::Movie(MovieId("m1".into()));
        let m2 = TitleId::Movie(MovieId("m2".into()));
        let states = svc
            .title_states(&user(), &[m1.clone(), m2.clone()])
            .await
            .unwrap();

        assert_eq!(
            states[0].progress_percent, 70,
            "both versions of a title carry progress, and the grid must show the same \
             number the continue rail resumes from, which is the most recently touched one"
        );
        assert_eq!(
            states[1].progress_percent, 0,
            "a title the viewer never started has no progress bar"
        );
    }

    #[tokio::test]
    async fn title_states_ignores_a_version_that_was_never_started() {
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
                position_ms: 0,
                audio_track: None,
                subtitle: None,
                updated_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .unwrap();

        let states = svc
            .title_states(&user(), &[TitleId::Movie(MovieId("m1".into()))])
            .await
            .unwrap();

        assert_eq!(
            states[0].progress_percent, 0,
            "opening a title without watching any of it leaves a zero row behind, \
             and the continue rail skips those, so the grid must too"
        );
    }

    #[tokio::test]
    async fn title_states_survives_progress_against_a_deleted_version() {
        let progress = Arc::new(MockProgressRepo::new());
        let svc = UserLibraryServiceImpl::new(
            Arc::clone(&progress),
            Arc::new(MockPreferencesRepo::new()),
            Arc::new(MockCatalogRepo::new()),
        );
        progress
            .upsert(PlaybackProgress {
                user: user(),
                version: VersionId("gone".into()),
                position_ms: 500,
                audio_track: None,
                subtitle: None,
                updated_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .unwrap();

        let states = svc
            .title_states(&user(), &[TitleId::Movie(MovieId("m1".into()))])
            .await
            .unwrap();

        assert_eq!(
            states[0].progress_percent, 0,
            "a pruned version leaves its progress row behind, and one stale row \
             must not fail the whole grid page"
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
            watched_ids(&progress).await,
            HashSet::from(["m1", "e9", "e1", "e2", "e3"].map(String::from))
        );
        assert!(svc.history(&user(), page()).await.unwrap().items.is_empty());
    }

    #[tokio::test]
    async fn marking_watched_takes_exactly_that_target_off_the_watchlist() {
        let preferences = Arc::new(MockPreferencesRepo::new());
        let catalog = Arc::new(MockCatalogRepo::new());
        catalog.add_series(series("sr1"));
        catalog.add_season(season("se1", "sr1"));
        catalog.add_season(season("se2", "sr1"));
        catalog.add_episode(episode("e1", "se1"));
        catalog.add_episode(episode("e2", "se1"));
        catalog.add_episode(episode("e3", "se2"));
        let svc = UserLibraryServiceImpl::new(
            Arc::new(MockProgressRepo::new()),
            Arc::clone(&preferences),
            Arc::clone(&catalog),
        );
        for title in [
            TitleId::Movie(MovieId("m1".into())),
            TitleId::Episode(EpisodeId("e1".into())),
            TitleId::Episode(EpisodeId("e2".into())),
            TitleId::Episode(EpisodeId("e3".into())),
        ] {
            svc.add_to_watchlist(&user(), &title).await.unwrap();
        }

        svc.set_watched(&user(), &WatchTarget::Episode(EpisodeId("e1".into())), true)
            .await
            .unwrap();

        assert_eq!(
            saved_ids(&svc).await,
            HashSet::from(["m1", "e2", "e3"].map(String::from)),
            "one episode watched takes only that episode off"
        );

        svc.set_watched(&user(), &WatchTarget::Season(SeasonId("se1".into())), true)
            .await
            .unwrap();

        assert_eq!(
            saved_ids(&svc).await,
            HashSet::from(["m1", "e3"].map(String::from)),
            "a season takes its own episodes and leaves the other season alone"
        );

        svc.set_watched(&user(), &WatchTarget::Series(SeriesId("sr1".into())), true)
            .await
            .unwrap();

        assert_eq!(
            saved_ids(&svc).await,
            HashSet::from(["m1".to_owned()]),
            "the series sweeps its episodes and never touches a movie"
        );
    }

    #[tokio::test]
    async fn marking_unwatched_leaves_the_watchlist_alone() {
        let svc = service();
        let m1 = TitleId::Movie(MovieId("m1".into()));
        svc.add_to_watchlist(&user(), &m1).await.unwrap();

        svc.set_watched(&user(), &WatchTarget::Movie(MovieId("m1".into())), false)
            .await
            .unwrap();

        assert_eq!(
            saved_ids(&svc).await,
            HashSet::from(["m1".to_owned()]),
            "clearing the flag must not be a way to lose a saved title"
        );

        svc.set_watched(&user(), &WatchTarget::Movie(MovieId("m1".into())), true)
            .await
            .unwrap();

        assert!(
            saved_ids(&svc).await.is_empty(),
            "watching it again after an unwatch removes it again"
        );
    }

    #[tokio::test]
    async fn set_unwatched_clears_flags() {
        let progress = Arc::new(MockProgressRepo::new());
        let svc = UserLibraryServiceImpl::new(
            Arc::clone(&progress),
            Arc::new(MockPreferencesRepo::new()),
            Arc::new(MockCatalogRepo::new()),
        );
        svc.set_watched(&user(), &WatchTarget::Movie(MovieId("m1".into())), true)
            .await
            .unwrap();
        assert_eq!(watched_ids(&progress).await.len(), 1);

        svc.set_watched(&user(), &WatchTarget::Movie(MovieId("m1".into())), false)
            .await
            .unwrap();
        assert!(watched_ids(&progress).await.is_empty());
        assert!(svc.history(&user(), page()).await.unwrap().items.is_empty());
    }

    #[tokio::test]
    async fn history_holds_views_and_survives_unmarking() {
        let progress = Arc::new(MockProgressRepo::new());
        let svc = UserLibraryServiceImpl::new(
            Arc::clone(&progress),
            Arc::new(MockPreferencesRepo::new()),
            Arc::new(MockCatalogRepo::new()),
        );
        progress
            .record_view(&user(), &title(), Timestamp::UNIX_EPOCH)
            .await
            .unwrap();
        progress
            .record_view(&user(), &title(), Timestamp::UNIX_EPOCH)
            .await
            .unwrap();

        svc.set_watched(&user(), &WatchTarget::Movie(MovieId("m1".into())), false)
            .await
            .unwrap();
        let history = svc.history(&user(), page()).await.unwrap();
        assert_eq!(history.total, 1);
        assert_eq!(history.items[0].play_count, 2);
        assert!(!history.items[0].watched);

        svc.remove_from_history(&user(), "m1").await.unwrap();
        assert!(svc.history(&user(), page()).await.unwrap().items.is_empty());
    }

    #[tokio::test]
    async fn clearing_history_keeps_what_is_marked_watched() {
        let progress = Arc::new(MockProgressRepo::new());
        let svc = UserLibraryServiceImpl::new(
            Arc::clone(&progress),
            Arc::new(MockPreferencesRepo::new()),
            Arc::new(MockCatalogRepo::new()),
        );
        progress
            .record_view(&user(), &title(), Timestamp::UNIX_EPOCH)
            .await
            .unwrap();

        svc.clear_history(&user()).await.unwrap();
        assert!(svc.history(&user(), page()).await.unwrap().items.is_empty());
        assert_eq!(watched_ids(&progress).await.len(), 1);
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
            progress.seed_history(WatchHistory {
                user: user(),
                title: TitleId::Episode(EpisodeId(id.into())),
                watched: true,
                play_count: 1,
                last_watched_at: None,
                completed: false,
            });
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
    async fn watched_rollups_resolve_a_whole_grid_page_in_two_reads() {
        let catalog = Arc::new(MockCatalogRepo::new());
        for s in 0..50 {
            catalog.add_series(series(&format!("sr{s}")));
            for n in 0..5 {
                let id = format!("se{s}-{n}");
                catalog.add_season(season(&id, &format!("sr{s}")));
                for e in 0..10 {
                    catalog.add_episode(episode(&format!("e{id}-{e}"), &id));
                }
            }
        }
        let svc = UserLibraryServiceImpl::new(
            Arc::new(MockProgressRepo::new()),
            Arc::new(MockPreferencesRepo::new()),
            Arc::clone(&catalog),
        );
        let targets: Vec<WatchTarget> = (0..50)
            .map(|s| WatchTarget::Series(SeriesId(format!("sr{s}"))))
            .collect();

        let rollups = svc.watched_rollups(&user(), &targets).await.unwrap();

        assert_eq!(rollups.len(), 50);
        assert!(
            rollups.iter().all(|r| r.total_episodes == 50),
            "every series must still count all fifty of its episodes"
        );
        assert_eq!(
            catalog.episode_lookup_count(),
            2,
            "a grid page resolves its whole batch in one read per kind of target; \
             walking seasons per series was 300 reads for this page"
        );
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

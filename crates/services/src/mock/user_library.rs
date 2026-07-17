use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use domain::catalog::{TitleId, VersionId};
use domain::common::{Page, PageRequest};
use domain::error::UserError;
use domain::playback::{
    Favorite, PlaybackProgress, TitleState, WatchHistory, WatchTarget, WatchedRollup, WatchlistItem,
};
use domain::service::UserLibraryService;
use domain::user::UserId;
use jiff::Timestamp;

use crate::page::paginate;

#[derive(Debug, Default)]
struct State {
    watchlist: HashMap<UserId, Vec<WatchlistItem>>,
    favorites: HashMap<UserId, Vec<Favorite>>,
    history: HashMap<UserId, Vec<WatchHistory>>,
    progress: HashMap<(UserId, VersionId), PlaybackProgress>,
}

#[derive(Clone, Default)]
pub struct MockUserLibraryService {
    state: Arc<Mutex<State>>,
}

impl MockUserLibraryService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_history(&self, item: WatchHistory) {
        self.state
            .lock()
            .unwrap()
            .history
            .entry(item.user.clone())
            .or_default()
            .push(item);
    }

    pub fn set_progress(&self, progress: PlaybackProgress) {
        self.state
            .lock()
            .unwrap()
            .progress
            .insert((progress.user.clone(), progress.version.clone()), progress);
    }
}

impl UserLibraryService for MockUserLibraryService {
    async fn watchlist(&self, user: &UserId) -> Result<Vec<WatchlistItem>, UserError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .watchlist
            .get(user)
            .cloned()
            .unwrap_or_default())
    }

    async fn add_to_watchlist(&self, user: &UserId, title: &TitleId) -> Result<(), UserError> {
        let mut state = self.state.lock().unwrap();
        let items = state.watchlist.entry(user.clone()).or_default();
        if !items.iter().any(|i| &i.title == title) {
            items.push(WatchlistItem {
                user: user.clone(),
                title: title.clone(),
                added_at: Timestamp::now(),
            });
        }
        Ok(())
    }

    async fn remove_from_watchlist(&self, user: &UserId, title_id: &str) -> Result<(), UserError> {
        if let Some(items) = self.state.lock().unwrap().watchlist.get_mut(user) {
            items.retain(|i| i.title.id() != title_id);
        }
        Ok(())
    }

    async fn favorites(&self, user: &UserId) -> Result<Vec<Favorite>, UserError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .favorites
            .get(user)
            .cloned()
            .unwrap_or_default())
    }

    async fn add_favorite(&self, user: &UserId, title: &TitleId) -> Result<(), UserError> {
        let mut state = self.state.lock().unwrap();
        let items = state.favorites.entry(user.clone()).or_default();
        if !items.iter().any(|i| &i.title == title) {
            items.push(Favorite {
                user: user.clone(),
                title: title.clone(),
                added_at: Timestamp::now(),
            });
        }
        Ok(())
    }

    async fn remove_favorite(&self, user: &UserId, title_id: &str) -> Result<(), UserError> {
        if let Some(items) = self.state.lock().unwrap().favorites.get_mut(user) {
            items.retain(|i| i.title.id() != title_id);
        }
        Ok(())
    }

    async fn history(
        &self,
        user: &UserId,
        page: PageRequest,
    ) -> Result<Page<WatchHistory>, UserError> {
        let state = self.state.lock().unwrap();
        let all = state.history.get(user).cloned().unwrap_or_default();
        Ok(paginate(&all, page))
    }

    async fn progress(
        &self,
        user: &UserId,
        version: &VersionId,
    ) -> Result<Option<PlaybackProgress>, UserError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .progress
            .get(&(user.clone(), version.clone()))
            .cloned())
    }

    async fn clear_progress(&self, user: &UserId, version: &VersionId) -> Result<(), UserError> {
        self.state
            .lock()
            .unwrap()
            .progress
            .remove(&(user.clone(), version.clone()));
        Ok(())
    }

    async fn set_watched(
        &self,
        user: &UserId,
        target: &WatchTarget,
        watched: bool,
    ) -> Result<(), UserError> {
        let title = match target {
            WatchTarget::Movie(id) => Some(TitleId::Movie(id.clone())),
            WatchTarget::Episode(id) => Some(TitleId::Episode(id.clone())),
            WatchTarget::Season(_) | WatchTarget::Series(_) => None,
        };
        if let Some(title) = title {
            let mut state = self.state.lock().unwrap();
            let entries = state.history.entry(user.clone()).or_default();
            entries.retain(|h| h.title != title);
            entries.push(WatchHistory {
                user: user.clone(),
                title,
                watched,
                play_count: u32::from(watched),
                last_watched_at: watched.then(Timestamp::now),
                completed: watched,
            });
        }
        Ok(())
    }

    async fn title_states(
        &self,
        user: &UserId,
        titles: &[TitleId],
    ) -> Result<Vec<TitleState>, UserError> {
        let state = self.state.lock().unwrap();
        let favorites: HashSet<TitleId> = state
            .favorites
            .get(user)
            .into_iter()
            .flatten()
            .map(|f| f.title.clone())
            .collect();
        let watchlisted: HashSet<TitleId> = state
            .watchlist
            .get(user)
            .into_iter()
            .flatten()
            .map(|w| w.title.clone())
            .collect();
        let history: HashMap<TitleId, (bool, bool)> = state
            .history
            .get(user)
            .into_iter()
            .flatten()
            .map(|h| (h.title.clone(), (h.watched, h.completed)))
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
        let state = self.state.lock().unwrap();
        let history: HashMap<TitleId, (bool, bool)> = state
            .history
            .get(user)
            .into_iter()
            .flatten()
            .map(|h| (h.title.clone(), (h.watched, h.completed)))
            .collect();
        Ok(targets
            .iter()
            .map(|target| {
                let leaves: Vec<TitleId> = match target {
                    WatchTarget::Movie(id) => vec![TitleId::Movie(id.clone())],
                    WatchTarget::Episode(id) => vec![TitleId::Episode(id.clone())],
                    WatchTarget::Season(_) | WatchTarget::Series(_) => Vec::new(),
                };
                let total = leaves.len() as u32;
                let mut watched_episodes = 0;
                let mut all_completed = true;
                for title in &leaves {
                    let (watched, completed) =
                        history.get(title).copied().unwrap_or((false, false));
                    if watched {
                        watched_episodes += 1;
                    }
                    all_completed &= completed;
                }
                WatchedRollup {
                    target: target.clone(),
                    watched: total > 0 && watched_episodes == total,
                    completed: total > 0 && all_completed,
                    watched_episodes,
                    total_episodes: total,
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{EpisodeId, MovieId, SeasonId, SeriesId};

    fn user() -> UserId {
        UserId("u1".into())
    }

    fn title() -> TitleId {
        TitleId::Movie(MovieId("m1".into()))
    }

    fn page(limit: u32) -> PageRequest {
        PageRequest { offset: 0, limit }
    }

    #[tokio::test]
    async fn watchlist_add_dedupe_remove() {
        let svc = MockUserLibraryService::new();
        assert!(svc.watchlist(&user()).await.unwrap().is_empty());
        svc.remove_from_watchlist(&user(), "m1").await.unwrap();

        svc.add_to_watchlist(&user(), &title()).await.unwrap();
        svc.add_to_watchlist(&user(), &title()).await.unwrap();
        assert_eq!(svc.watchlist(&user()).await.unwrap().len(), 1);

        svc.remove_from_watchlist(&user(), "m1").await.unwrap();
        assert!(svc.watchlist(&user()).await.unwrap().is_empty());
        svc.remove_from_watchlist(&user(), "m1").await.unwrap();
    }

    #[tokio::test]
    async fn favorites_add_dedupe_remove() {
        let svc = MockUserLibraryService::new();
        assert!(svc.favorites(&user()).await.unwrap().is_empty());
        svc.remove_favorite(&user(), "m1").await.unwrap();

        svc.add_favorite(&user(), &title()).await.unwrap();
        svc.add_favorite(&user(), &title()).await.unwrap();
        assert_eq!(svc.favorites(&user()).await.unwrap().len(), 1);

        svc.remove_favorite(&user(), "m1").await.unwrap();
        assert!(svc.favorites(&user()).await.unwrap().is_empty());
        svc.remove_favorite(&user(), "m1").await.unwrap();
    }

    #[tokio::test]
    async fn history_paginated() {
        let svc = MockUserLibraryService::new();
        for _ in 0..3 {
            svc.add_history(WatchHistory {
                user: user(),
                title: title(),
                watched: true,
                play_count: 1,
                last_watched_at: None,
                completed: true,
            });
        }
        let page = svc
            .history(
                &user(),
                PageRequest {
                    offset: 0,
                    limit: 2,
                },
            )
            .await
            .unwrap();
        assert_eq!(page.total, 3);
        assert_eq!(page.items.len(), 2);
    }

    #[tokio::test]
    async fn progress_get() {
        let svc = MockUserLibraryService::new();
        let version = VersionId("v1".into());
        assert!(svc.progress(&user(), &version).await.unwrap().is_none());

        svc.set_progress(PlaybackProgress {
            user: user(),
            version: version.clone(),
            position_ms: 1000,
            updated_at: Timestamp::now(),
        });
        let progress = svc.progress(&user(), &version).await.unwrap().unwrap();
        assert_eq!(progress.position_ms, 1000);

        svc.clear_progress(&user(), &version).await.unwrap();
        assert!(svc.progress(&user(), &version).await.unwrap().is_none());
        svc.clear_progress(&user(), &version).await.unwrap();
    }

    #[tokio::test]
    async fn set_watched_records_titles_and_ignores_containers() {
        let svc = MockUserLibraryService::new();
        svc.set_watched(&user(), &WatchTarget::Movie(MovieId("m1".into())), true)
            .await
            .unwrap();
        svc.set_watched(&user(), &WatchTarget::Episode(EpisodeId("e1".into())), true)
            .await
            .unwrap();
        svc.set_watched(&user(), &WatchTarget::Season(SeasonId("se1".into())), true)
            .await
            .unwrap();
        svc.set_watched(&user(), &WatchTarget::Series(SeriesId("sr1".into())), true)
            .await
            .unwrap();
        let listed = svc.history(&user(), page(10)).await.unwrap();
        assert_eq!(listed.total, 2);
        assert!(
            listed
                .items
                .iter()
                .all(|h| h.completed && h.play_count == 1)
        );

        svc.set_watched(&user(), &WatchTarget::Movie(MovieId("m1".into())), false)
            .await
            .unwrap();
        let listed = svc.history(&user(), page(10)).await.unwrap();
        assert_eq!(listed.total, 2);
        let movie = listed.items.iter().find(|h| h.title.id() == "m1").unwrap();
        assert!(!movie.watched && !movie.completed && movie.play_count == 0);
        assert!(movie.last_watched_at.is_none());
    }

    #[tokio::test]
    async fn title_states_reflects_flags_and_preserves_order() {
        let svc = MockUserLibraryService::new();
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
        assert!(svc.title_states(&user(), &[]).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn watched_rollups_count_leaves_and_skip_containers() {
        let svc = MockUserLibraryService::new();
        svc.set_watched(&user(), &WatchTarget::Movie(MovieId("m1".into())), true)
            .await
            .unwrap();

        let rollups = svc
            .watched_rollups(
                &user(),
                &[
                    WatchTarget::Movie(MovieId("m1".into())),
                    WatchTarget::Episode(EpisodeId("e1".into())),
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
                    target: WatchTarget::Movie(MovieId("m1".into())),
                    watched: true,
                    completed: true,
                    watched_episodes: 1,
                    total_episodes: 1,
                },
                WatchedRollup {
                    target: WatchTarget::Episode(EpisodeId("e1".into())),
                    watched: false,
                    completed: false,
                    watched_episodes: 0,
                    total_episodes: 1,
                },
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
}

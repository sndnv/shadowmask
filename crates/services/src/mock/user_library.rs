use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use domain::catalog::{TitleId, VersionId};
use domain::common::{Page, PageRequest};
use domain::error::UserError;
use domain::playback::{Favorite, PlaybackProgress, WatchHistory, WatchlistItem};
use domain::service::UserLibraryService;
use domain::user::UserId;
use jiff::Timestamp;

use crate::mock::page::paginate;

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

    async fn remove_from_watchlist(&self, user: &UserId, title: &TitleId) -> Result<(), UserError> {
        if let Some(items) = self.state.lock().unwrap().watchlist.get_mut(user) {
            items.retain(|i| &i.title != title);
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

    async fn remove_favorite(&self, user: &UserId, title: &TitleId) -> Result<(), UserError> {
        if let Some(items) = self.state.lock().unwrap().favorites.get_mut(user) {
            items.retain(|i| &i.title != title);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::MovieId;

    fn user() -> UserId {
        UserId("u1".into())
    }

    fn title() -> TitleId {
        TitleId::Movie(MovieId("m1".into()))
    }

    #[tokio::test]
    async fn watchlist_add_dedupe_remove() {
        let svc = MockUserLibraryService::new();
        assert!(svc.watchlist(&user()).await.unwrap().is_empty());
        svc.remove_from_watchlist(&user(), &title()).await.unwrap();

        svc.add_to_watchlist(&user(), &title()).await.unwrap();
        svc.add_to_watchlist(&user(), &title()).await.unwrap();
        assert_eq!(svc.watchlist(&user()).await.unwrap().len(), 1);

        svc.remove_from_watchlist(&user(), &title()).await.unwrap();
        assert!(svc.watchlist(&user()).await.unwrap().is_empty());
        svc.remove_from_watchlist(&user(), &title()).await.unwrap();
    }

    #[tokio::test]
    async fn favorites_add_dedupe_remove() {
        let svc = MockUserLibraryService::new();
        assert!(svc.favorites(&user()).await.unwrap().is_empty());
        svc.remove_favorite(&user(), &title()).await.unwrap();

        svc.add_favorite(&user(), &title()).await.unwrap();
        svc.add_favorite(&user(), &title()).await.unwrap();
        assert_eq!(svc.favorites(&user()).await.unwrap().len(), 1);

        svc.remove_favorite(&user(), &title()).await.unwrap();
        assert!(svc.favorites(&user()).await.unwrap().is_empty());
        svc.remove_favorite(&user(), &title()).await.unwrap();
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
    }
}

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use domain::catalog::{TitleId, VersionId};
use domain::common::{Page, PageRequest};
use domain::error::RepositoryError;
use domain::playback::{PlaybackProgress, WatchHistory};
use domain::repository::ProgressRepository;
use domain::user::UserId;
use jiff::Timestamp;

use domain::common::paginate;

#[derive(Default)]
struct State {
    progress: Vec<PlaybackProgress>,
    history: Vec<WatchHistory>,
}

fn prune(history: &mut Vec<WatchHistory>) {
    history.retain(|h| h.play_count > 0 || h.watched);
}

#[derive(Clone, Default)]
pub struct MockProgressRepo {
    state: Arc<Mutex<State>>,
    fail: Arc<AtomicBool>,
}

impl MockProgressRepo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_fail(&self) {
        self.fail.store(true, Ordering::Relaxed);
    }

    pub fn seed_history(&self, history: WatchHistory) {
        self.state.lock().unwrap().history.push(history);
    }

    pub fn seed_progress(&self, progress: PlaybackProgress) {
        let mut state = self.state.lock().unwrap();
        state
            .progress
            .retain(|p| !(p.user == progress.user && p.version == progress.version));
        state.progress.push(progress);
    }

    fn guard(&self) -> Result<(), RepositoryError> {
        if self.fail.load(Ordering::Relaxed) {
            Err(RepositoryError::Backend("mock progress failure".to_owned()))
        } else {
            Ok(())
        }
    }
}

impl ProgressRepository for MockProgressRepo {
    async fn get(
        &self,
        user: &UserId,
        version: &VersionId,
    ) -> Result<Option<PlaybackProgress>, RepositoryError> {
        self.guard()?;
        Ok(self
            .state
            .lock()
            .unwrap()
            .progress
            .iter()
            .find(|p| &p.user == user && &p.version == version)
            .cloned())
    }

    async fn upsert(&self, progress: PlaybackProgress) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if let Some(existing) = state
            .progress
            .iter_mut()
            .find(|p| p.user == progress.user && p.version == progress.version)
        {
            *existing = progress;
        } else {
            state.progress.push(progress);
        }
        Ok(())
    }

    async fn delete(&self, user: &UserId, version: &VersionId) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state
            .lock()
            .unwrap()
            .progress
            .retain(|p| !(&p.user == user && &p.version == version));
        Ok(())
    }

    async fn list_in_progress(
        &self,
        user: &UserId,
    ) -> Result<Vec<PlaybackProgress>, RepositoryError> {
        self.guard()?;
        Ok(self
            .state
            .lock()
            .unwrap()
            .progress
            .iter()
            .filter(|p| &p.user == user)
            .cloned()
            .collect())
    }

    async fn record_view(
        &self,
        user: &UserId,
        title: &TitleId,
        at: Timestamp,
    ) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if let Some(existing) = state
            .history
            .iter_mut()
            .find(|h| &h.user == user && &h.title == title)
        {
            existing.watched = true;
            existing.completed = true;
            existing.play_count += 1;
            existing.last_watched_at = Some(at);
        } else {
            state.history.push(WatchHistory {
                user: user.clone(),
                title: title.clone(),
                watched: true,
                play_count: 1,
                last_watched_at: Some(at),
                completed: true,
            });
        }
        Ok(())
    }

    async fn set_watched_flags(
        &self,
        user: &UserId,
        title: &TitleId,
        watched: bool,
    ) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if let Some(existing) = state
            .history
            .iter_mut()
            .find(|h| &h.user == user && &h.title == title)
        {
            existing.watched = watched;
            existing.completed = watched;
        } else {
            state.history.push(WatchHistory {
                user: user.clone(),
                title: title.clone(),
                watched,
                play_count: 0,
                last_watched_at: None,
                completed: watched,
            });
        }
        prune(&mut state.history);
        Ok(())
    }

    async fn delete_history(&self, user: &UserId, title_id: &str) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        for entry in state
            .history
            .iter_mut()
            .filter(|h| &h.user == user && h.title.id() == title_id)
        {
            entry.play_count = 0;
            entry.last_watched_at = None;
        }
        prune(&mut state.history);
        Ok(())
    }

    async fn clear_history(&self, user: &UserId) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        for entry in state.history.iter_mut().filter(|h| &h.user == user) {
            entry.play_count = 0;
            entry.last_watched_at = None;
        }
        prune(&mut state.history);
        Ok(())
    }

    async fn watched_state(&self, user: &UserId) -> Result<Vec<WatchHistory>, RepositoryError> {
        self.guard()?;
        Ok(self
            .state
            .lock()
            .unwrap()
            .history
            .iter()
            .filter(|h| &h.user == user)
            .cloned()
            .collect())
    }

    async fn history(
        &self,
        user: &UserId,
        page: PageRequest,
    ) -> Result<Page<WatchHistory>, RepositoryError> {
        self.guard()?;
        let all: Vec<WatchHistory> = self
            .state
            .lock()
            .unwrap()
            .history
            .iter()
            .filter(|h| &h.user == user && h.play_count > 0)
            .cloned()
            .collect();
        Ok(paginate(&all, page))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::MovieId;
    use domain::catalog::TitleId;
    use jiff::Timestamp;

    fn user() -> UserId {
        UserId("u1".to_owned())
    }

    fn progress(version: &str, position_ms: u64) -> PlaybackProgress {
        PlaybackProgress {
            user: user(),
            version: VersionId(version.to_owned()),
            position_ms,
            audio_track: None,
            subtitle: None,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn title() -> TitleId {
        TitleId::Movie(MovieId("m1".to_owned()))
    }

    fn page() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 10,
        }
    }

    #[tokio::test]
    async fn progress_upsert_get_and_list() {
        let repo = MockProgressRepo::new();
        assert!(
            repo.get(&user(), &VersionId("v1".into()))
                .await
                .unwrap()
                .is_none()
        );
        repo.upsert(progress("v1", 100)).await.unwrap();
        repo.upsert(progress("v1", 250)).await.unwrap();
        assert_eq!(
            repo.get(&user(), &VersionId("v1".into()))
                .await
                .unwrap()
                .unwrap()
                .position_ms,
            250
        );
        assert_eq!(repo.list_in_progress(&user()).await.unwrap().len(), 1);

        repo.delete(&user(), &VersionId("v1".into())).await.unwrap();
        assert!(
            repo.get(&user(), &VersionId("v1".into()))
                .await
                .unwrap()
                .is_none()
        );
        assert!(repo.list_in_progress(&user()).await.unwrap().is_empty());
        repo.delete(&user(), &VersionId("v1".into())).await.unwrap();
    }

    #[tokio::test]
    async fn views_accumulate_and_paginate() {
        let repo = MockProgressRepo::new();
        repo.record_view(&user(), &title(), Timestamp::UNIX_EPOCH)
            .await
            .unwrap();
        repo.record_view(&user(), &title(), Timestamp::UNIX_EPOCH)
            .await
            .unwrap();
        let page = repo.history(&user(), page()).await.unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].play_count, 2);
        assert!(page.items[0].completed);
    }

    #[tokio::test]
    async fn marking_watched_never_reaches_the_view_history() {
        let repo = MockProgressRepo::new();
        repo.set_watched_flags(&user(), &title(), true)
            .await
            .unwrap();
        assert_eq!(repo.history(&user(), page()).await.unwrap().total, 0);
        assert!(repo.watched_state(&user()).await.unwrap()[0].watched);

        repo.set_watched_flags(&user(), &title(), false)
            .await
            .unwrap();
        assert!(repo.watched_state(&user()).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn removing_a_view_keeps_the_watched_flag() {
        let repo = MockProgressRepo::new();
        repo.record_view(&user(), &title(), Timestamp::UNIX_EPOCH)
            .await
            .unwrap();
        repo.delete_history(&user(), "m1").await.unwrap();
        assert_eq!(repo.history(&user(), page()).await.unwrap().total, 0);
        assert!(repo.watched_state(&user()).await.unwrap()[0].watched);

        repo.record_view(&user(), &title(), Timestamp::UNIX_EPOCH)
            .await
            .unwrap();
        repo.clear_history(&user()).await.unwrap();
        assert_eq!(repo.history(&user(), page()).await.unwrap().total, 0);
    }

    #[tokio::test]
    async fn surfaces_backend_failure() {
        let repo = MockProgressRepo::new();
        repo.set_fail();
        assert!(repo.get(&user(), &VersionId("v1".into())).await.is_err());
        assert!(repo.upsert(progress("v1", 1)).await.is_err());
        assert!(repo.delete(&user(), &VersionId("v1".into())).await.is_err());
        assert!(repo.list_in_progress(&user()).await.is_err());
        assert!(
            repo.record_view(&user(), &title(), Timestamp::UNIX_EPOCH)
                .await
                .is_err()
        );
        assert!(
            repo.set_watched_flags(&user(), &title(), true)
                .await
                .is_err()
        );
        assert!(repo.delete_history(&user(), "m1").await.is_err());
        assert!(repo.clear_history(&user()).await.is_err());
        assert!(repo.watched_state(&user()).await.is_err());
        assert!(repo.history(&user(), page()).await.is_err());
    }
}

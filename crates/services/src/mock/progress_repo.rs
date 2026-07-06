use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use domain::catalog::VersionId;
use domain::common::{Page, PageRequest};
use domain::error::RepositoryError;
use domain::playback::{PlaybackProgress, WatchHistory};
use domain::repository::ProgressRepository;
use domain::user::UserId;

use crate::page::paginate;

#[derive(Default)]
struct State {
    progress: Vec<PlaybackProgress>,
    history: Vec<WatchHistory>,
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

    async fn record_history(&self, history: WatchHistory) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if let Some(existing) = state
            .history
            .iter_mut()
            .find(|h| h.user == history.user && h.title == history.title)
        {
            *existing = history;
        } else {
            state.history.push(history);
        }
        Ok(())
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
            .filter(|h| &h.user == user)
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
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn history(completed: bool) -> WatchHistory {
        WatchHistory {
            user: user(),
            title: TitleId::Movie(MovieId("m1".to_owned())),
            watched: true,
            play_count: 1,
            last_watched_at: None,
            completed,
        }
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
    }

    #[tokio::test]
    async fn history_record_and_paginate() {
        let repo = MockProgressRepo::new();
        repo.record_history(history(false)).await.unwrap();
        repo.record_history(history(true)).await.unwrap();
        let page = repo.history(&user(), page()).await.unwrap();
        assert_eq!(page.total, 1);
        assert!(page.items[0].completed);
    }

    #[tokio::test]
    async fn surfaces_backend_failure() {
        let repo = MockProgressRepo::new();
        repo.set_fail();
        assert!(repo.get(&user(), &VersionId("v1".into())).await.is_err());
        assert!(repo.upsert(progress("v1", 1)).await.is_err());
        assert!(repo.list_in_progress(&user()).await.is_err());
        assert!(repo.record_history(history(false)).await.is_err());
        assert!(repo.history(&user(), page()).await.is_err());
    }
}

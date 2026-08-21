use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use domain::catalog::VersionId;
use domain::error::RepositoryError;
use domain::playback::{Favorite, SubtitleTrackRef, UserSubtitleOffset, WatchlistItem};
use domain::repository::PreferencesRepository;
use domain::user::UserId;

#[derive(Default)]
struct State {
    watchlist: Vec<WatchlistItem>,
    favorites: Vec<Favorite>,
    offsets: Vec<UserSubtitleOffset>,
}

#[derive(Clone, Default)]
pub struct MockPreferencesRepo {
    state: Arc<Mutex<State>>,
    fail: Arc<AtomicBool>,
}

impl MockPreferencesRepo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed_watchlist(&self, item: WatchlistItem) {
        self.state.lock().unwrap().watchlist.push(item);
    }

    pub fn set_fail(&self) {
        self.fail.store(true, Ordering::Relaxed);
    }

    fn guard(&self) -> Result<(), RepositoryError> {
        if self.fail.load(Ordering::Relaxed) {
            Err(RepositoryError::Backend(
                "mock preferences failure".to_owned(),
            ))
        } else {
            Ok(())
        }
    }
}

impl PreferencesRepository for MockPreferencesRepo {
    async fn list_watchlist(&self, user: &UserId) -> Result<Vec<WatchlistItem>, RepositoryError> {
        self.guard()?;
        Ok(self
            .state
            .lock()
            .unwrap()
            .watchlist
            .iter()
            .filter(|i| &i.user == user)
            .cloned()
            .collect())
    }

    async fn add_watchlist(&self, item: WatchlistItem) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if !state
            .watchlist
            .iter()
            .any(|i| i.user == item.user && i.title == item.title)
        {
            state.watchlist.push(item);
        }
        Ok(())
    }

    async fn remove_watchlist(&self, user: &UserId, title_id: &str) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state
            .lock()
            .unwrap()
            .watchlist
            .retain(|i| !(&i.user == user && i.title.id() == title_id));
        Ok(())
    }

    async fn list_favorites(&self, user: &UserId) -> Result<Vec<Favorite>, RepositoryError> {
        self.guard()?;
        Ok(self
            .state
            .lock()
            .unwrap()
            .favorites
            .iter()
            .filter(|i| &i.user == user)
            .cloned()
            .collect())
    }

    async fn add_favorite(&self, item: Favorite) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if !state
            .favorites
            .iter()
            .any(|i| i.user == item.user && i.title == item.title)
        {
            state.favorites.push(item);
        }
        Ok(())
    }

    async fn remove_favorite(&self, user: &UserId, title_id: &str) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state
            .lock()
            .unwrap()
            .favorites
            .retain(|i| !(&i.user == user && i.title.id() == title_id));
        Ok(())
    }

    async fn get_subtitle_offset(
        &self,
        user: &UserId,
        version: &VersionId,
        subtitle: &SubtitleTrackRef,
    ) -> Result<Option<UserSubtitleOffset>, RepositoryError> {
        self.guard()?;
        Ok(self
            .state
            .lock()
            .unwrap()
            .offsets
            .iter()
            .find(|o| &o.user == user && &o.version == version && &o.subtitle == subtitle)
            .cloned())
    }

    async fn set_subtitle_offset(&self, offset: UserSubtitleOffset) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if let Some(existing) = state.offsets.iter_mut().find(|o| {
            o.user == offset.user && o.version == offset.version && o.subtitle == offset.subtitle
        }) {
            *existing = offset;
        } else {
            state.offsets.push(offset);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{MovieId, TitleId};
    use jiff::Timestamp;

    fn user() -> UserId {
        UserId("u1".to_owned())
    }

    fn title() -> TitleId {
        TitleId::Movie(MovieId("m1".to_owned()))
    }

    fn watchlist_item() -> WatchlistItem {
        WatchlistItem {
            user: user(),
            title: title(),
            added_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn favorite() -> Favorite {
        Favorite {
            user: user(),
            title: title(),
            added_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn offset() -> UserSubtitleOffset {
        UserSubtitleOffset {
            user: user(),
            version: VersionId("v1".to_owned()),
            subtitle: SubtitleTrackRef::Embedded(1),
            offset_ms: 500,
        }
    }

    #[tokio::test]
    async fn watchlist_add_dedupe_remove() {
        let repo = MockPreferencesRepo::new();
        assert!(repo.list_watchlist(&user()).await.unwrap().is_empty());
        repo.add_watchlist(watchlist_item()).await.unwrap();
        repo.add_watchlist(watchlist_item()).await.unwrap();
        assert_eq!(repo.list_watchlist(&user()).await.unwrap().len(), 1);
        repo.remove_watchlist(&user(), "m1").await.unwrap();
        assert!(repo.list_watchlist(&user()).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn favorites_add_dedupe_remove() {
        let repo = MockPreferencesRepo::new();
        repo.add_favorite(favorite()).await.unwrap();
        repo.add_favorite(favorite()).await.unwrap();
        assert_eq!(repo.list_favorites(&user()).await.unwrap().len(), 1);
        repo.remove_favorite(&user(), "m1").await.unwrap();
        assert!(repo.list_favorites(&user()).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn subtitle_offset_set_and_get() {
        let repo = MockPreferencesRepo::new();
        let version = VersionId("v1".to_owned());
        let track = SubtitleTrackRef::Embedded(1);
        assert!(
            repo.get_subtitle_offset(&user(), &version, &track)
                .await
                .unwrap()
                .is_none()
        );
        repo.set_subtitle_offset(offset()).await.unwrap();
        let mut updated = offset();
        updated.offset_ms = 750;
        repo.set_subtitle_offset(updated).await.unwrap();
        assert_eq!(
            repo.get_subtitle_offset(&user(), &version, &track)
                .await
                .unwrap()
                .unwrap()
                .offset_ms,
            750
        );
    }

    #[tokio::test]
    async fn surfaces_backend_failure() {
        let repo = MockPreferencesRepo::new();
        repo.set_fail();
        assert!(repo.list_watchlist(&user()).await.is_err());
        assert!(repo.add_watchlist(watchlist_item()).await.is_err());
        assert!(repo.remove_watchlist(&user(), "m1").await.is_err());
        assert!(repo.list_favorites(&user()).await.is_err());
        assert!(repo.add_favorite(favorite()).await.is_err());
        assert!(repo.remove_favorite(&user(), "m1").await.is_err());
        assert!(
            repo.get_subtitle_offset(
                &user(),
                &VersionId("v1".into()),
                &SubtitleTrackRef::Embedded(1)
            )
            .await
            .is_err()
        );
        assert!(repo.set_subtitle_offset(offset()).await.is_err());
    }
}

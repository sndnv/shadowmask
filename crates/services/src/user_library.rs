use std::sync::Arc;

use domain::catalog::{TitleId, VersionId};
use domain::common::{Page, PageRequest};
use domain::error::UserError;
use domain::playback::{Favorite, PlaybackProgress, WatchHistory, WatchlistItem};
use domain::repository::{PreferencesRepository, ProgressRepository};
use domain::service::UserLibraryService;
use domain::user::UserId;
use jiff::Timestamp;

pub struct UserLibraryServiceImpl<Pr, Pf> {
    progress: Arc<Pr>,
    preferences: Arc<Pf>,
}

impl<Pr, Pf> UserLibraryServiceImpl<Pr, Pf> {
    pub fn new(progress: Arc<Pr>, preferences: Arc<Pf>) -> Self {
        Self {
            progress,
            preferences,
        }
    }
}

impl<Pr, Pf> Clone for UserLibraryServiceImpl<Pr, Pf> {
    fn clone(&self) -> Self {
        Self {
            progress: Arc::clone(&self.progress),
            preferences: Arc::clone(&self.preferences),
        }
    }
}

impl<Pr, Pf> UserLibraryService for UserLibraryServiceImpl<Pr, Pf>
where
    Pr: ProgressRepository + Send + Sync,
    Pf: PreferencesRepository + Send + Sync,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{MockPreferencesRepo, MockProgressRepo};
    use domain::catalog::MovieId;

    fn service() -> UserLibraryServiceImpl<MockProgressRepo, MockPreferencesRepo> {
        UserLibraryServiceImpl::new(
            Arc::new(MockProgressRepo::new()),
            Arc::new(MockPreferencesRepo::new()),
        )
    }

    fn user() -> UserId {
        UserId("u1".into())
    }

    fn title() -> TitleId {
        TitleId::Movie(MovieId("m1".into()))
    }

    fn page() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 10,
        }
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
        let svc = UserLibraryServiceImpl::new(Arc::new(progress), Arc::new(preferences));

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
    }
}

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use domain::common::{Page, PageRequest};
use domain::error::RepositoryError;
use domain::repository::SessionRegistry;
use domain::session::{PlaybackSession, SessionId};
use domain::user::UserId;

use crate::page::paginate;

#[derive(Clone, Default)]
pub struct MockSessionRegistry {
    sessions: Arc<Mutex<HashMap<SessionId, PlaybackSession>>>,
    fail: Arc<AtomicBool>,
}

impl MockSessionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_fail(&self) {
        self.fail.store(true, Ordering::Relaxed);
    }

    fn guard(&self) -> Result<(), RepositoryError> {
        if self.fail.load(Ordering::Relaxed) {
            Err(RepositoryError::Backend("mock session failure".to_owned()))
        } else {
            Ok(())
        }
    }

    fn sorted(&self) -> Vec<PlaybackSession> {
        let mut items: Vec<PlaybackSession> =
            self.sessions.lock().unwrap().values().cloned().collect();
        items.sort_by(|a, b| {
            a.started_at
                .cmp(&b.started_at)
                .then_with(|| a.id.0.cmp(&b.id.0))
        });
        items
    }
}

impl SessionRegistry for MockSessionRegistry {
    async fn insert(&self, session: PlaybackSession) -> Result<(), RepositoryError> {
        self.guard()?;
        self.sessions
            .lock()
            .unwrap()
            .insert(session.id.clone(), session);
        Ok(())
    }

    async fn get(&self, id: &SessionId) -> Result<Option<PlaybackSession>, RepositoryError> {
        self.guard()?;
        Ok(self.sessions.lock().unwrap().get(id).cloned())
    }

    async fn list_for_user(&self, user: &UserId) -> Result<Vec<PlaybackSession>, RepositoryError> {
        self.guard()?;
        Ok(self
            .sorted()
            .into_iter()
            .filter(|s| &s.user == user)
            .collect())
    }

    async fn list_all(&self, page: PageRequest) -> Result<Page<PlaybackSession>, RepositoryError> {
        self.guard()?;
        Ok(paginate(&self.sorted(), page))
    }

    async fn remove(&self, id: &SessionId) -> Result<(), RepositoryError> {
        self.guard()?;
        self.sessions.lock().unwrap().remove(id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::VersionId;
    use domain::session::{DeliveryMode, PlaybackState, SelectedTracks};
    use jiff::Timestamp;

    fn session(id: &str, user: &str, offset: i64) -> PlaybackSession {
        PlaybackSession {
            id: SessionId(id.to_owned()),
            user: UserId(user.to_owned()),
            device: None,
            version: VersionId("v1".to_owned()),
            mode: DeliveryMode::Direct,
            position_ms: 0,
            state: PlaybackState::Playing,
            selected: SelectedTracks {
                audio_track: None,
                subtitle_track: None,
                subtitle_delivery: None,
            },
            started_at: Timestamp::from_second(1_700_000_000 + offset).unwrap(),
            last_heartbeat_at: Timestamp::from_second(1_700_000_000 + offset).unwrap(),
        }
    }

    fn page() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 10,
        }
    }

    #[tokio::test]
    async fn insert_get_list_remove() {
        let registry = MockSessionRegistry::new();
        registry.insert(session("s2", "u1", 2)).await.unwrap();
        registry.insert(session("s1", "u1", 1)).await.unwrap();
        registry.insert(session("s3", "u2", 3)).await.unwrap();

        assert!(
            registry
                .get(&SessionId("s1".into()))
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            registry
                .get(&SessionId("x".into()))
                .await
                .unwrap()
                .is_none()
        );

        let mine = registry.list_for_user(&UserId("u1".into())).await.unwrap();
        let ids: Vec<String> = mine.iter().map(|s| s.id.0.clone()).collect();
        assert_eq!(ids, ["s1", "s2"]);

        assert_eq!(registry.list_all(page()).await.unwrap().total, 3);

        registry.remove(&SessionId("s1".into())).await.unwrap();
        assert!(
            registry
                .get(&SessionId("s1".into()))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn surfaces_backend_failure() {
        let registry = MockSessionRegistry::new();
        registry.set_fail();
        assert!(registry.insert(session("s1", "u1", 0)).await.is_err());
        assert!(registry.get(&SessionId("s1".into())).await.is_err());
        assert!(registry.list_for_user(&UserId("u1".into())).await.is_err());
        assert!(registry.list_all(page()).await.is_err());
        assert!(registry.remove(&SessionId("s1".into())).await.is_err());
    }
}

use std::collections::HashMap;
use std::sync::RwLock;

use domain::common::{Page, PageRequest, paginate};
use domain::error::RepositoryError;
use domain::repository::SessionRegistry;
use domain::session::{PlaybackSession, SessionId};
use domain::user::UserId;

#[derive(Default)]
pub struct InMemorySessionRegistry {
    sessions: RwLock<HashMap<SessionId, PlaybackSession>>,
}

impl InMemorySessionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn sorted(&self) -> Vec<PlaybackSession> {
        let mut items: Vec<PlaybackSession> =
            self.sessions.read().unwrap().values().cloned().collect();
        items.sort_by(|a, b| {
            a.started_at
                .cmp(&b.started_at)
                .then_with(|| a.id.0.cmp(&b.id.0))
        });
        items
    }
}

impl SessionRegistry for InMemorySessionRegistry {
    async fn insert(&self, session: PlaybackSession) -> Result<(), RepositoryError> {
        self.sessions
            .write()
            .unwrap()
            .insert(session.id.clone(), session);
        Ok(())
    }

    async fn get(&self, id: &SessionId) -> Result<Option<PlaybackSession>, RepositoryError> {
        Ok(self.sessions.read().unwrap().get(id).cloned())
    }

    async fn list_for_user(&self, user: &UserId) -> Result<Vec<PlaybackSession>, RepositoryError> {
        Ok(self
            .sorted()
            .into_iter()
            .filter(|s| &s.user == user)
            .collect())
    }

    async fn list_all(&self, page: PageRequest) -> Result<Page<PlaybackSession>, RepositoryError> {
        Ok(paginate(&self.sorted(), page))
    }

    async fn remove(&self, id: &SessionId) -> Result<(), RepositoryError> {
        self.sessions.write().unwrap().remove(id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::VersionId;
    use domain::session::{DeliveryMode, PlaybackState, SelectedTracks};
    use jiff::Timestamp;

    fn session(id: &str, user: &str, started_offset: i64) -> PlaybackSession {
        PlaybackSession {
            id: SessionId(id.into()),
            user: UserId(user.into()),
            device: None,
            version: VersionId("v1".into()),
            mode: DeliveryMode::Direct,
            position_ms: 0,
            state: PlaybackState::Playing,
            selected: SelectedTracks {
                audio_track: None,
                subtitle_track: None,
                subtitle_delivery: None,
            },
            started_at: Timestamp::from_second(1_700_000_000 + started_offset).unwrap(),
            last_heartbeat_at: Timestamp::from_second(1_700_000_000 + started_offset).unwrap(),
        }
    }

    fn page(offset: u32, limit: u32) -> PageRequest {
        PageRequest { offset, limit }
    }

    #[tokio::test]
    async fn insert_get_remove() {
        let registry = InMemorySessionRegistry::new();
        registry.insert(session("s1", "u1", 0)).await.unwrap();
        let found = registry.get(&SessionId("s1".into())).await.unwrap();
        assert_eq!(found.unwrap().id, SessionId("s1".into()));
        assert!(
            registry
                .get(&SessionId("missing".into()))
                .await
                .unwrap()
                .is_none()
        );
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
    async fn list_for_user_filters_and_orders() {
        let registry = InMemorySessionRegistry::new();
        registry.insert(session("s2", "u1", 2)).await.unwrap();
        registry.insert(session("s1", "u1", 1)).await.unwrap();
        registry.insert(session("s3", "u2", 3)).await.unwrap();
        let mine = registry.list_for_user(&UserId("u1".into())).await.unwrap();
        let ids: Vec<String> = mine.iter().map(|s| s.id.0.clone()).collect();
        assert_eq!(ids, ["s1", "s2"]);
    }

    #[tokio::test]
    async fn list_all_paginates() {
        let registry = InMemorySessionRegistry::new();
        for (i, id) in ["s1", "s2", "s3"].iter().enumerate() {
            registry.insert(session(id, "u1", i as i64)).await.unwrap();
        }
        let all = registry.list_all(page(0, 10)).await.unwrap();
        assert_eq!(all.total, 3);
        let second = registry.list_all(page(1, 1)).await.unwrap();
        assert_eq!(second.total, 3);
        assert_eq!(second.items[0].id, SessionId("s2".into()));
    }
}

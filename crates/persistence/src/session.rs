use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use domain::common::{Page, PageRequest, paginate};
use domain::error::RepositoryError;
use domain::repository::SessionRegistry;
use domain::session::{DeliveryMode, PlaybackSession, SessionId};
use domain::user::{DeviceId, UserId};
use jiff::Timestamp;

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
        items.sort_by(|a, b| a.started_at.cmp(&b.started_at).then_with(|| a.id.0.cmp(&b.id.0)));
        items
    }
}

fn record_gauges(sessions: &HashMap<SessionId, PlaybackSession>) {
    let mut direct = 0u64;
    let mut remux = 0u64;
    let mut transcode = 0u64;
    let mut users: HashSet<&UserId> = HashSet::new();
    let mut devices: HashSet<&DeviceId> = HashSet::new();
    for session in sessions.values() {
        match session.mode {
            DeliveryMode::Direct => direct += 1,
            DeliveryMode::Remux => remux += 1,
            DeliveryMode::Transcode => transcode += 1,
        }
        users.insert(&session.user);
        if let Some(device) = &session.device {
            devices.insert(device);
        }
    }
    metrics::gauge!("sessions_active", "mode" => "direct").set(direct as f64);
    metrics::gauge!("sessions_active", "mode" => "remux").set(remux as f64);
    metrics::gauge!("sessions_active", "mode" => "transcode").set(transcode as f64);
    metrics::gauge!("active_users").set(users.len() as f64);
    metrics::gauge!("active_devices").set(devices.len() as f64);
}

impl SessionRegistry for InMemorySessionRegistry {
    async fn insert(&self, session: PlaybackSession) -> Result<(), RepositoryError> {
        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(session.id.clone(), session);
        record_gauges(&sessions);
        Ok(())
    }

    async fn get(&self, id: &SessionId) -> Result<Option<PlaybackSession>, RepositoryError> {
        Ok(self.sessions.read().unwrap().get(id).cloned())
    }

    async fn list_for_user(&self, user: &UserId) -> Result<Vec<PlaybackSession>, RepositoryError> {
        Ok(self.sorted().into_iter().filter(|s| &s.user == user).collect())
    }

    async fn list_all(&self, page: PageRequest) -> Result<Page<PlaybackSession>, RepositoryError> {
        Ok(paginate(&self.sorted(), page))
    }

    async fn remove(&self, id: &SessionId) -> Result<(), RepositoryError> {
        let mut sessions = self.sessions.write().unwrap();
        sessions.remove(id);
        record_gauges(&sessions);
        Ok(())
    }

    async fn remove_idle(&self, cutoff: Timestamp) -> Result<usize, RepositoryError> {
        let mut sessions = self.sessions.write().unwrap();
        let before = sessions.len();
        sessions.retain(|_, s| s.last_heartbeat_at >= cutoff);
        let removed = before - sessions.len();
        if removed > 0 {
            record_gauges(&sessions);
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::VersionId;
    use domain::session::{PlaybackState, SelectedTracks};
    use jiff::Timestamp;
    use metrics_exporter_prometheus::PrometheusBuilder;

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
            completed: false,
        }
    }

    fn session_mode(
        id: &str,
        user: &str,
        mode: DeliveryMode,
        device: Option<&str>,
    ) -> PlaybackSession {
        PlaybackSession { device: device.map(|d| DeviceId(d.into())), mode, ..session(id, user, 0) }
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
        assert!(registry.get(&SessionId("missing".into())).await.unwrap().is_none());
        registry.remove(&SessionId("s1".into())).await.unwrap();
        assert!(registry.get(&SessionId("s1".into())).await.unwrap().is_none());
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

    #[tokio::test]
    async fn remove_idle_drops_only_what_stopped_heartbeating() {
        let registry = InMemorySessionRegistry::new();
        for (i, id) in ["stale", "borderline", "live"].iter().enumerate() {
            registry.insert(session(id, "u1", i as i64)).await.unwrap();
        }
        let cutoff = Timestamp::from_second(1_700_000_001).unwrap();

        let dropped = registry.remove_idle(cutoff).await.unwrap();

        assert_eq!(dropped, 1);
        let left = registry.list_all(page(0, 10)).await.unwrap();
        assert_eq!(left.total, 2);
        assert!(!left.items.iter().any(|s| s.id == SessionId("stale".into())));
    }

    #[tokio::test]
    async fn remove_idle_with_nothing_to_drop_reports_zero() {
        let registry = InMemorySessionRegistry::new();
        registry.insert(session("live", "u1", 10)).await.unwrap();
        let cutoff = Timestamp::from_second(1_700_000_000).unwrap();

        assert_eq!(registry.remove_idle(cutoff).await.unwrap(), 0);
        assert_eq!(registry.list_all(page(0, 10)).await.unwrap().total, 1);
    }

    fn recorded<F, Fut>(work: F) -> String
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let recorder = PrometheusBuilder::new().build_recorder();
        let handle = recorder.handle();
        metrics::with_local_recorder(&recorder, || {
            let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
            rt.block_on(work());
        });
        handle.render()
    }

    #[test]
    fn insert_records_session_gauges() {
        let rendered = recorded(|| async {
            let registry = InMemorySessionRegistry::new();
            registry.insert(session_mode("s1", "u1", DeliveryMode::Direct, None)).await.unwrap();
            registry
                .insert(session_mode("s2", "u1", DeliveryMode::Remux, Some("d1")))
                .await
                .unwrap();
            registry
                .insert(session_mode("s3", "u2", DeliveryMode::Transcode, Some("d1")))
                .await
                .unwrap();
        });
        assert!(rendered.contains("sessions_active{mode=\"direct\"} 1"));
        assert!(rendered.contains("sessions_active{mode=\"remux\"} 1"));
        assert!(rendered.contains("sessions_active{mode=\"transcode\"} 1"));
        assert!(rendered.contains("active_users 2"));
        assert!(rendered.contains("active_devices 1"));
    }

    #[test]
    fn remove_updates_session_gauges() {
        let rendered = recorded(|| async {
            let registry = InMemorySessionRegistry::new();
            registry.insert(session_mode("s1", "u1", DeliveryMode::Direct, None)).await.unwrap();
            registry.remove(&SessionId("s1".into())).await.unwrap();
        });
        assert!(rendered.contains("sessions_active{mode=\"direct\"} 0"));
        assert!(rendered.contains("active_users 0"));
    }
}

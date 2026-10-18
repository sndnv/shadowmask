use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use domain::common::{Page, PageRequest, paginate};
use domain::error::RepositoryError;
use domain::repository::SessionRegistry;
use domain::session::{DeliveryMode, PlaybackSession, PlaybackState, SessionId};
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
    let mut active = [0u64; 3];
    let mut playing = [0u64; 3];
    let mut users: HashSet<&UserId> = HashSet::new();
    let mut devices: HashSet<&DeviceId> = HashSet::new();
    for session in sessions.values() {
        let slot = mode_slot(session.mode);
        active[slot] += 1;
        if session.state == PlaybackState::Playing {
            playing[slot] += 1;
        }
        users.insert(&session.user);
        if let Some(device) = &session.device {
            devices.insert(device);
        }
    }
    for mode in [DeliveryMode::Direct, DeliveryMode::Remux, DeliveryMode::Transcode] {
        let slot = mode_slot(mode);
        metrics::gauge!("sessions_active", "mode" => mode_label(mode)).set(active[slot] as f64);
        metrics::gauge!("sessions_playing", "mode" => mode_label(mode)).set(playing[slot] as f64);
    }
    metrics::gauge!("active_users").set(users.len() as f64);
    metrics::gauge!("active_devices").set(devices.len() as f64);
}

fn mode_slot(mode: DeliveryMode) -> usize {
    match mode {
        DeliveryMode::Direct => 0,
        DeliveryMode::Remux => 1,
        DeliveryMode::Transcode => 2,
    }
}

fn mode_label(mode: DeliveryMode) -> &'static str {
    match mode {
        DeliveryMode::Direct => "direct",
        DeliveryMode::Remux => "remux",
        DeliveryMode::Transcode => "transcode",
    }
}

impl SessionRegistry for InMemorySessionRegistry {
    async fn insert(&self, session: PlaybackSession) -> Result<(), RepositoryError> {
        let mut sessions = self.sessions.write().unwrap();
        let mode = session.mode;
        if sessions.insert(session.id.clone(), session).is_none() {
            metrics::counter!("sessions_started_total", "mode" => mode_label(mode)).increment(1);
        }
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

    async fn list_idle(&self, cutoff: Timestamp) -> Result<Vec<SessionId>, RepositoryError> {
        Ok(self
            .sorted()
            .into_iter()
            .filter(|s| s.last_heartbeat_at < cutoff)
            .map(|s| s.id)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::VersionId;
    use domain::session::SelectedTracks;
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
    async fn list_idle_names_only_what_stopped_heartbeating_and_keeps_it() {
        let registry = InMemorySessionRegistry::new();
        for (i, id) in ["stale", "borderline", "live"].iter().enumerate() {
            registry.insert(session(id, "u1", i as i64)).await.unwrap();
        }
        let cutoff = Timestamp::from_second(1_700_000_001).unwrap();

        let idle = registry.list_idle(cutoff).await.unwrap();

        assert_eq!(idle, vec![SessionId("stale".into())]);
        assert_eq!(registry.list_all(page(0, 10)).await.unwrap().total, 3);
    }

    #[tokio::test]
    async fn list_idle_with_nothing_idle_names_none() {
        let registry = InMemorySessionRegistry::new();
        registry.insert(session("live", "u1", 10)).await.unwrap();
        let cutoff = Timestamp::from_second(1_700_000_000).unwrap();

        assert!(registry.list_idle(cutoff).await.unwrap().is_empty());
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
    fn only_sessions_that_last_reported_playing_count_as_playing() {
        let rendered = recorded(|| async {
            let registry = InMemorySessionRegistry::new();
            registry.insert(session_mode("s1", "u1", DeliveryMode::Transcode, None)).await.unwrap();
            registry
                .insert(PlaybackSession {
                    state: PlaybackState::Paused,
                    ..session_mode("s2", "u1", DeliveryMode::Transcode, None)
                })
                .await
                .unwrap();
            registry
                .insert(PlaybackSession {
                    state: PlaybackState::Paused,
                    ..session_mode("s3", "u2", DeliveryMode::Direct, None)
                })
                .await
                .unwrap();
        });
        assert!(rendered.contains("sessions_active{mode=\"transcode\"} 2"));
        assert!(rendered.contains("sessions_playing{mode=\"transcode\"} 1"));
        assert!(rendered.contains("sessions_active{mode=\"direct\"} 1"));
        assert!(rendered.contains("sessions_playing{mode=\"direct\"} 0"));
        assert!(rendered.contains("sessions_playing{mode=\"remux\"} 0"));
    }

    #[test]
    fn a_new_session_counts_once_by_its_mode() {
        let rendered = recorded(|| async {
            let registry = InMemorySessionRegistry::new();
            registry.insert(session_mode("s1", "u1", DeliveryMode::Direct, None)).await.unwrap();
            registry.insert(session_mode("s2", "u1", DeliveryMode::Remux, None)).await.unwrap();
            registry.insert(session_mode("s3", "u2", DeliveryMode::Transcode, None)).await.unwrap();
            registry.insert(session_mode("s4", "u2", DeliveryMode::Transcode, None)).await.unwrap();
            registry.insert(session_mode("s1", "u1", DeliveryMode::Transcode, None)).await.unwrap();
        });
        assert!(rendered.contains("sessions_started_total{mode=\"direct\"} 1"));
        assert!(rendered.contains("sessions_started_total{mode=\"remux\"} 1"));
        assert!(rendered.contains("sessions_started_total{mode=\"transcode\"} 2"));
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

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use domain::error::SessionError;
use domain::service::SessionService;
use domain::session::{
    DeliveryMode, HeartbeatAck, PlaybackSession, PlaybackState, Renegotiated, SelectedTracks,
    SessionId, SessionStarted, SessionUpdate, StartSessionRequest, SubtitleChange,
    SubtitleDelivery,
};
use domain::user::UserId;
use jiff::Timestamp;

const HEARTBEAT_INTERVAL_S: u32 = 10;

#[derive(Debug, Default)]
struct State {
    sessions: HashMap<SessionId, PlaybackSession>,
    next_id: u64,
}

#[derive(Clone, Default)]
pub struct MockSessionService {
    state: Arc<Mutex<State>>,
}

impl MockSessionService {
    pub fn new() -> Self {
        Self::default()
    }
}

fn manifest_url(id: &SessionId) -> String {
    format!("/stream/{}/master.m3u8", id.0)
}

fn renegotiated(session: &PlaybackSession) -> Renegotiated {
    Renegotiated {
        session_id: session.id.clone(),
        mode: session.mode,
        manifest_url: manifest_url(&session.id),
        selected: session.selected.clone(),
    }
}

impl SessionService for MockSessionService {
    async fn start(
        &self,
        user: &UserId,
        request: StartSessionRequest,
    ) -> Result<SessionStarted, SessionError> {
        let mut state = self.state.lock().unwrap();
        state.next_id += 1;
        let id = SessionId(format!("session-{}", state.next_id));
        let now = Timestamp::now();
        let selected = SelectedTracks {
            audio_track: request.audio_track,
            subtitle_track: request.subtitle.as_ref().map(|s| s.track.clone()),
            subtitle_delivery: request.subtitle.as_ref().map(|_| SubtitleDelivery::HlsVtt),
        };
        let session = PlaybackSession {
            id: id.clone(),
            user: user.clone(),
            device: None,
            version: request.version,
            mode: DeliveryMode::Direct,
            position_ms: request.start_position_ms,
            state: PlaybackState::Playing,
            selected: selected.clone(),
            started_at: now,
            last_heartbeat_at: now,
        };
        state.sessions.insert(id.clone(), session);
        Ok(SessionStarted {
            session_id: id.clone(),
            mode: DeliveryMode::Direct,
            manifest_url: manifest_url(&id),
            selected,
            heartbeat_interval_s: HEARTBEAT_INTERVAL_S,
        })
    }

    async fn heartbeat(
        &self,
        session: &SessionId,
        position_ms: u64,
        state: PlaybackState,
    ) -> Result<HeartbeatAck, SessionError> {
        let mut guard = self.state.lock().unwrap();
        let s = guard
            .sessions
            .get_mut(session)
            .ok_or(SessionError::NotFound)?;
        s.position_ms = position_ms;
        s.state = state;
        s.last_heartbeat_at = Timestamp::now();
        Ok(HeartbeatAck {
            heartbeat_interval_s: HEARTBEAT_INTERVAL_S,
        })
    }

    async fn seek(
        &self,
        session: &SessionId,
        position_ms: u64,
    ) -> Result<Renegotiated, SessionError> {
        let mut guard = self.state.lock().unwrap();
        let s = guard
            .sessions
            .get_mut(session)
            .ok_or(SessionError::NotFound)?;
        s.position_ms = position_ms;
        s.last_heartbeat_at = Timestamp::now();
        Ok(renegotiated(s))
    }

    async fn update(
        &self,
        session: &SessionId,
        update: SessionUpdate,
    ) -> Result<Renegotiated, SessionError> {
        let mut guard = self.state.lock().unwrap();
        let s = guard
            .sessions
            .get_mut(session)
            .ok_or(SessionError::NotFound)?;
        if let Some(audio) = update.audio_track {
            s.selected.audio_track = Some(audio);
        }
        match update.subtitle {
            SubtitleChange::Keep => {}
            SubtitleChange::Disable => {
                s.selected.subtitle_track = None;
                s.selected.subtitle_delivery = None;
            }
            SubtitleChange::Set(selection) => {
                s.selected.subtitle_track = Some(selection.track);
                s.selected.subtitle_delivery = Some(SubtitleDelivery::HlsVtt);
            }
        }
        Ok(renegotiated(s))
    }

    async fn end(&self, session: &SessionId) -> Result<(), SessionError> {
        self.state
            .lock()
            .unwrap()
            .sessions
            .remove(session)
            .map(|_| ())
            .ok_or(SessionError::NotFound)
    }

    async fn active_sessions(&self) -> Result<Vec<PlaybackSession>, SessionError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .sessions
            .values()
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::VersionId;
    use domain::playback::SubtitleTrackRef;
    use domain::session::{ClientCapabilities, SubtitleSelection};

    fn start_request() -> StartSessionRequest {
        StartSessionRequest {
            version: VersionId("v1".into()),
            start_position_ms: 0,
            capabilities: ClientCapabilities {
                platform: "web".into(),
                profile_version: 1,
                max_bitrate: None,
            },
            audio_track: Some(0),
            subtitle: None,
        }
    }

    async fn started_session() -> (MockSessionService, SessionId) {
        let svc = MockSessionService::new();
        let started = svc
            .start(&UserId("u1".into()), start_request())
            .await
            .unwrap();
        (svc, started.session_id)
    }

    #[tokio::test]
    async fn start_creates_session() {
        let (svc, id) = started_session().await;
        assert_eq!(id, SessionId("session-1".into()));
        let active = svc.active_sessions().await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].mode, DeliveryMode::Direct);
    }

    #[tokio::test]
    async fn start_with_subtitle_selection() {
        let svc = MockSessionService::new();
        let request = StartSessionRequest {
            subtitle: Some(SubtitleSelection {
                track: SubtitleTrackRef::Embedded(2),
                offset_ms: Some(500),
            }),
            ..start_request()
        };
        let started = svc.start(&UserId("u1".into()), request).await.unwrap();
        assert_eq!(
            started.selected.subtitle_track,
            Some(SubtitleTrackRef::Embedded(2))
        );
        assert_eq!(
            started.selected.subtitle_delivery,
            Some(SubtitleDelivery::HlsVtt)
        );
    }

    #[tokio::test]
    async fn heartbeat_updates_position() {
        let (svc, id) = started_session().await;
        let ack = svc
            .heartbeat(&id, 5000, PlaybackState::Paused)
            .await
            .unwrap();
        assert_eq!(ack.heartbeat_interval_s, HEARTBEAT_INTERVAL_S);
        let active = svc.active_sessions().await.unwrap();
        assert_eq!(active[0].position_ms, 5000);
        assert_eq!(active[0].state, PlaybackState::Paused);

        assert!(matches!(
            svc.heartbeat(&SessionId("x".into()), 0, PlaybackState::Playing)
                .await
                .unwrap_err(),
            SessionError::NotFound
        ));
    }

    #[tokio::test]
    async fn seek_updates_position() {
        let (svc, id) = started_session().await;
        let nego = svc.seek(&id, 12345).await.unwrap();
        assert_eq!(nego.session_id, id);
        assert_eq!(svc.active_sessions().await.unwrap()[0].position_ms, 12345);

        assert!(matches!(
            svc.seek(&SessionId("x".into()), 0).await.unwrap_err(),
            SessionError::NotFound
        ));
    }

    #[tokio::test]
    async fn update_changes_tracks() {
        let (svc, id) = started_session().await;

        svc.update(
            &id,
            SessionUpdate {
                audio_track: None,
                subtitle: SubtitleChange::Keep,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            svc.active_sessions().await.unwrap()[0].selected.audio_track,
            Some(0)
        );

        svc.update(
            &id,
            SessionUpdate {
                audio_track: Some(2),
                subtitle: SubtitleChange::Set(SubtitleSelection {
                    track: SubtitleTrackRef::Embedded(1),
                    offset_ms: None,
                }),
            },
        )
        .await
        .unwrap();
        let selected = svc.active_sessions().await.unwrap()[0].selected.clone();
        assert_eq!(selected.audio_track, Some(2));
        assert_eq!(selected.subtitle_track, Some(SubtitleTrackRef::Embedded(1)));
        assert_eq!(selected.subtitle_delivery, Some(SubtitleDelivery::HlsVtt));

        svc.update(
            &id,
            SessionUpdate {
                audio_track: None,
                subtitle: SubtitleChange::Disable,
            },
        )
        .await
        .unwrap();
        let selected = svc.active_sessions().await.unwrap()[0].selected.clone();
        assert_eq!(selected.subtitle_track, None);
        assert_eq!(selected.subtitle_delivery, None);

        assert!(matches!(
            svc.update(
                &SessionId("x".into()),
                SessionUpdate {
                    audio_track: None,
                    subtitle: SubtitleChange::Keep,
                },
            )
            .await
            .unwrap_err(),
            SessionError::NotFound
        ));
    }

    #[tokio::test]
    async fn end_removes_session() {
        let (svc, id) = started_session().await;
        svc.end(&id).await.unwrap();
        assert!(svc.active_sessions().await.unwrap().is_empty());
        assert!(matches!(
            svc.end(&id).await.unwrap_err(),
            SessionError::NotFound
        ));
    }
}

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use domain::common::{Page, PageRequest};
use domain::error::SessionError;
use domain::media::{CreditsMarker, DetectedMarkers, IntroMarker, TrickplayAsset};
use domain::service::SessionService;
use domain::session::{
    DeliveryMode, HeartbeatAck, PlaybackSession, PlaybackState, Renegotiated, SelectedTracks,
    SessionId, SessionStarted, SessionUpdate, StartSessionRequest, SubtitleChange,
    SubtitleDelivery,
};
use domain::user::Principal;
use jiff::Timestamp;
use uuid::Uuid;

use crate::page::paginate;

const HEARTBEAT_INTERVAL_S: u32 = 10;

#[derive(Debug, Default)]
struct State {
    sessions: HashMap<SessionId, PlaybackSession>,
    concurrent_limit: Option<usize>,
}

#[derive(Clone, Default)]
pub struct MockSessionService {
    state: Arc<Mutex<State>>,
}

impl MockSessionService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_concurrent_limit(&self, limit: usize) {
        self.state.lock().unwrap().concurrent_limit = Some(limit);
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
        caller: &Principal,
        request: StartSessionRequest,
    ) -> Result<SessionStarted, SessionError> {
        let user = &caller.user;
        let mut state = self.state.lock().unwrap();
        if let Some(limit) = state.concurrent_limit {
            let active: Vec<PlaybackSession> = state
                .sessions
                .values()
                .filter(|s| &s.user == user)
                .cloned()
                .collect();
            if active.len() >= limit {
                return Err(SessionError::ConcurrentLimit { active });
            }
        }
        let id = SessionId(Uuid::new_v4().to_string());
        let now = Timestamp::now();
        let selected = SelectedTracks {
            audio_track: request.audio_track,
            subtitle_track: request.subtitle.as_ref().map(|s| s.track.clone()),
            subtitle_delivery: request.subtitle.as_ref().map(|_| SubtitleDelivery::HlsVtt),
        };
        let version = request.version.clone();
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
            markers: DetectedMarkers {
                intros: vec![IntroMarker {
                    version: version.clone(),
                    start_ms: 60_000,
                    end_ms: 90_000,
                }],
                credits: vec![CreditsMarker {
                    version: version.clone(),
                    start_ms: 900_000,
                    end_ms: 960_000,
                }],
            },
            trickplay: vec![TrickplayAsset {
                version,
                interval_ms: 10_000,
                columns: 5,
                rows: 5,
                tile_width: 320,
                tile_height: 180,
                sheet_paths: vec!["sheet-000.jpg".into(), "sheet-001.jpg".into()],
            }],
        })
    }

    async fn heartbeat(
        &self,
        _caller: &Principal,
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
        _caller: &Principal,
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
        _caller: &Principal,
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

    async fn end(&self, _caller: &Principal, session: &SessionId) -> Result<(), SessionError> {
        self.state
            .lock()
            .unwrap()
            .sessions
            .remove(session)
            .map(|_| ())
            .ok_or(SessionError::NotFound)
    }

    async fn active_sessions(
        &self,
        _caller: &Principal,
        page: PageRequest,
    ) -> Result<Page<PlaybackSession>, SessionError> {
        let all: Vec<PlaybackSession> = self
            .state
            .lock()
            .unwrap()
            .sessions
            .values()
            .cloned()
            .collect();
        Ok(paginate(&all, page))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::VersionId;
    use domain::playback::SubtitleTrackRef;
    use domain::session::{ClientCapabilities, SubtitleSelection};
    use domain::user::{Role, UserId};

    fn principal(user: &str) -> Principal {
        Principal {
            user: UserId(user.into()),
            role: Role::User,
        }
    }

    fn page() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 10,
        }
    }

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
        let started = svc.start(&principal("u1"), start_request()).await.unwrap();
        (svc, started.session_id)
    }

    #[tokio::test]
    async fn start_creates_session() {
        let (svc, id) = started_session().await;
        assert!(Uuid::parse_str(&id.0).is_ok());
        let active = svc.active_sessions(&principal("u1"), page()).await.unwrap();
        assert_eq!(active.total, 1);
        assert_eq!(active.items[0].mode, DeliveryMode::Direct);
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
        let started = svc.start(&principal("u1"), request).await.unwrap();
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
            .heartbeat(&principal("u1"), &id, 5000, PlaybackState::Paused)
            .await
            .unwrap();
        assert_eq!(ack.heartbeat_interval_s, HEARTBEAT_INTERVAL_S);
        let active = svc.active_sessions(&principal("u1"), page()).await.unwrap();
        assert_eq!(active.items[0].position_ms, 5000);
        assert_eq!(active.items[0].state, PlaybackState::Paused);

        assert!(matches!(
            svc.heartbeat(
                &principal("u1"),
                &SessionId("x".into()),
                0,
                PlaybackState::Playing
            )
            .await
            .unwrap_err(),
            SessionError::NotFound
        ));
    }

    #[tokio::test]
    async fn seek_updates_position() {
        let (svc, id) = started_session().await;
        let nego = svc.seek(&principal("u1"), &id, 12345).await.unwrap();
        assert_eq!(nego.session_id, id);
        assert_eq!(
            svc.active_sessions(&principal("u1"), page())
                .await
                .unwrap()
                .items[0]
                .position_ms,
            12345
        );

        assert!(matches!(
            svc.seek(&principal("u1"), &SessionId("x".into()), 0)
                .await
                .unwrap_err(),
            SessionError::NotFound
        ));
    }

    #[tokio::test]
    async fn update_changes_tracks() {
        let (svc, id) = started_session().await;

        svc.update(
            &principal("u1"),
            &id,
            SessionUpdate {
                audio_track: None,
                subtitle: SubtitleChange::Keep,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            svc.active_sessions(&principal("u1"), page())
                .await
                .unwrap()
                .items[0]
                .selected
                .audio_track,
            Some(0)
        );

        svc.update(
            &principal("u1"),
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
        let selected = svc
            .active_sessions(&principal("u1"), page())
            .await
            .unwrap()
            .items[0]
            .selected
            .clone();
        assert_eq!(selected.audio_track, Some(2));
        assert_eq!(selected.subtitle_track, Some(SubtitleTrackRef::Embedded(1)));
        assert_eq!(selected.subtitle_delivery, Some(SubtitleDelivery::HlsVtt));

        svc.update(
            &principal("u1"),
            &id,
            SessionUpdate {
                audio_track: None,
                subtitle: SubtitleChange::Disable,
            },
        )
        .await
        .unwrap();
        let selected = svc
            .active_sessions(&principal("u1"), page())
            .await
            .unwrap()
            .items[0]
            .selected
            .clone();
        assert_eq!(selected.subtitle_track, None);
        assert_eq!(selected.subtitle_delivery, None);

        assert!(matches!(
            svc.update(
                &principal("u1"),
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
    async fn start_enforces_concurrent_limit() {
        let svc = MockSessionService::new();
        svc.set_concurrent_limit(1);
        svc.start(&principal("u1"), start_request()).await.unwrap();
        assert!(matches!(
            svc.start(&principal("u1"), start_request())
                .await
                .unwrap_err(),
            SessionError::ConcurrentLimit { active } if active.len() == 1
        ));
        svc.start(&principal("u2"), start_request()).await.unwrap();
    }

    #[tokio::test]
    async fn end_removes_session() {
        let (svc, id) = started_session().await;
        svc.end(&principal("u1"), &id).await.unwrap();
        assert!(
            svc.active_sessions(&principal("u1"), page())
                .await
                .unwrap()
                .items
                .is_empty()
        );
        assert!(matches!(
            svc.end(&principal("u1"), &id).await.unwrap_err(),
            SessionError::NotFound
        ));
    }
}

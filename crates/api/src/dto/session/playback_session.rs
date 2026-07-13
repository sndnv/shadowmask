use serde::Serialize;

use domain::session::{NowPlaying, PlaybackSession};

use super::{DeliveryModeDto, PlaybackStateDto, SelectedTracksResponse};
use crate::dto::common::ResumeCardDto;

#[derive(Debug, Serialize)]
pub struct PlaybackSessionResponse {
    pub session_id: String,
    pub user_id: String,
    pub device_id: Option<String>,
    pub version_id: String,
    pub mode: DeliveryModeDto,
    pub position_ms: u64,
    pub state: PlaybackStateDto,
    pub selected: SelectedTracksResponse,
    pub started_at: String,
    pub last_heartbeat_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card: Option<ResumeCardDto>,
}

impl From<PlaybackSession> for PlaybackSessionResponse {
    fn from(s: PlaybackSession) -> Self {
        PlaybackSessionResponse {
            session_id: s.id.0,
            user_id: s.user.0,
            device_id: s.device.map(|d| d.0),
            version_id: s.version.0,
            mode: s.mode.into(),
            position_ms: s.position_ms,
            state: s.state.into(),
            selected: s.selected.into(),
            started_at: s.started_at.to_string(),
            last_heartbeat_at: s.last_heartbeat_at.to_string(),
            card: None,
        }
    }
}

impl From<NowPlaying> for PlaybackSessionResponse {
    fn from(n: NowPlaying) -> Self {
        let mut response = PlaybackSessionResponse::from(n.session);
        response.card = Some(n.card.into());
        response
    }
}

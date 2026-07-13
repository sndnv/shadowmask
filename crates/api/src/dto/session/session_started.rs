use serde::Serialize;

use domain::session::SessionStarted;

use super::{DeliveryModeDto, SelectedTracksResponse};
use crate::dto::catalog::{MarkersDto, TrickplayRefDto};

#[derive(Debug, Serialize)]
pub struct SessionStartedResponse {
    pub session_id: String,
    pub mode: DeliveryModeDto,
    pub manifest_url: String,
    pub selected: SelectedTracksResponse,
    pub heartbeat_interval_s: u32,
    pub markers: MarkersDto,
    pub trickplay: Vec<TrickplayRefDto>,
}

impl From<SessionStarted> for SessionStartedResponse {
    fn from(s: SessionStarted) -> Self {
        SessionStartedResponse {
            session_id: s.session_id.0,
            mode: s.mode.into(),
            manifest_url: s.manifest_url,
            selected: s.selected.into(),
            heartbeat_interval_s: s.heartbeat_interval_s,
            markers: s.markers.into(),
            trickplay: s.trickplay.into_iter().map(Into::into).collect(),
        }
    }
}

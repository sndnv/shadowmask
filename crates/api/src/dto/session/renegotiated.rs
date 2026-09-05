use serde::Serialize;

use domain::session::Renegotiated;

use super::{DeliveryModeDto, SelectedTracksResponse};

#[derive(Debug, Serialize)]
pub struct RenegotiatedResponse {
    pub session_id: String,
    pub mode: DeliveryModeDto,
    pub manifest_url: String,
    pub origin_ms: u64,
    pub sequential: bool,
    pub selected: SelectedTracksResponse,
}

impl From<Renegotiated> for RenegotiatedResponse {
    fn from(r: Renegotiated) -> Self {
        RenegotiatedResponse {
            session_id: r.session_id.0,
            mode: r.mode.into(),
            manifest_url: r.manifest_url,
            origin_ms: r.origin_ms,
            sequential: r.sequential,
            selected: r.selected.into(),
        }
    }
}

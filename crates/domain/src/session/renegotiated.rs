use crate::session::{DeliveryMode, SelectedTracks, SessionId};

#[derive(Debug, Clone)]
pub struct Renegotiated {
    pub session_id: SessionId,
    pub mode: DeliveryMode,
    pub manifest_url: String,
    pub selected: SelectedTracks,
}

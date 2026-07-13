use crate::media::{DetectedMarkers, TrickplayAsset};
use crate::session::{DeliveryMode, SelectedTracks, SessionId};

#[derive(Debug, Clone)]
pub struct SessionStarted {
    pub session_id: SessionId,
    pub mode: DeliveryMode,
    pub manifest_url: String,
    pub selected: SelectedTracks,
    pub heartbeat_interval_s: u32,
    pub markers: DetectedMarkers,
    pub trickplay: Vec<TrickplayAsset>,
}

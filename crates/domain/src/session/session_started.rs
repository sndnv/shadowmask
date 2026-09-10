use crate::media::{DetectedMarkers, TrickplayAsset};
use crate::session::{DeliveryMode, SegmentContainer, SelectedTracks, SessionId};

#[derive(Debug, Clone)]
pub struct SessionStarted {
    pub session_id: SessionId,
    pub mode: DeliveryMode,
    pub container: Option<SegmentContainer>,
    pub manifest_url: String,
    pub origin_ms: u64,
    pub sequential: bool,
    pub selected: SelectedTracks,
    pub heartbeat_interval_s: u32,
    pub markers: DetectedMarkers,
    pub trickplay: Vec<TrickplayAsset>,
}

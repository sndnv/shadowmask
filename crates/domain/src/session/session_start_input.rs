use crate::catalog::VersionId;
use crate::session::{AudioRequest, ClientCapabilities, DeliveryPreference, SubtitleRequest};

#[derive(Debug, Clone)]
pub struct SessionStartInput {
    pub version: VersionId,
    pub start_position_ms: u64,
    pub capabilities: ClientCapabilities,
    pub audio: AudioRequest,
    pub subtitle: SubtitleRequest,
    pub target_height: Option<u32>,
    pub force_burn: bool,
    pub downmix_stereo: bool,
    pub delivery: DeliveryPreference,
}

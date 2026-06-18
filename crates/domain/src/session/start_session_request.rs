use crate::catalog::VersionId;
use crate::session::{ClientCapabilities, SubtitleSelection};

#[derive(Debug, Clone)]
pub struct StartSessionRequest {
    pub version: VersionId,
    pub start_position_ms: u64,
    pub capabilities: ClientCapabilities,
    pub audio_track: Option<u32>,
    pub subtitle: Option<SubtitleSelection>,
}

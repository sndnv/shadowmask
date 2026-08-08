use serde::Deserialize;

use domain::catalog::VersionId;
use domain::session::SessionStartInput;

use super::{ClientCapabilitiesDto, SubtitleSelectionDto};

#[derive(Debug, Deserialize)]
pub struct StartSessionRequest {
    pub version_id: String,
    #[serde(default)]
    pub start_position_ms: u64,
    pub capabilities: ClientCapabilitiesDto,
    pub audio_track: Option<u32>,
    pub subtitle: Option<SubtitleSelectionDto>,
    pub target_height: Option<u32>,
    #[serde(default)]
    pub force_burn: bool,
    #[serde(default)]
    pub downmix_stereo: bool,
}

impl From<StartSessionRequest> for SessionStartInput {
    fn from(r: StartSessionRequest) -> Self {
        SessionStartInput {
            version: VersionId(r.version_id),
            start_position_ms: r.start_position_ms,
            capabilities: r.capabilities.into(),
            audio_track: r.audio_track,
            subtitle: r.subtitle.map(Into::into),
            target_height: r.target_height,
            force_burn: r.force_burn,
            downmix_stereo: r.downmix_stereo,
        }
    }
}

use serde::Deserialize;

use domain::catalog::VersionId;
use domain::session::StartSessionRequest;

use super::{ClientCapabilitiesDto, SubtitleSelectionDto};

#[derive(Debug, Deserialize)]
pub struct StartSessionRequestDto {
    pub version_id: String,
    #[serde(default)]
    pub start_position_ms: u64,
    pub capabilities: ClientCapabilitiesDto,
    pub audio_track: Option<u32>,
    pub subtitle: Option<SubtitleSelectionDto>,
}

impl From<StartSessionRequestDto> for StartSessionRequest {
    fn from(r: StartSessionRequestDto) -> Self {
        StartSessionRequest {
            version: VersionId(r.version_id),
            start_position_ms: r.start_position_ms,
            capabilities: r.capabilities.into(),
            audio_track: r.audio_track,
            subtitle: r.subtitle.map(Into::into),
        }
    }
}

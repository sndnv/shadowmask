use serde::Deserialize;

use domain::catalog::VersionId;
use domain::common::LanguageCode;
use domain::session::{AudioRequest, SessionStartInput, SubtitleRequest};

use super::{ClientCapabilitiesDto, SubtitleSelectionDto};

#[derive(Debug, Deserialize)]
pub struct StartSessionRequest {
    pub version_id: String,
    #[serde(default)]
    pub start_position_ms: u64,
    pub capabilities: ClientCapabilitiesDto,
    pub audio_track: Option<u32>,
    pub audio_language: Option<String>,
    pub subtitle: Option<SubtitleSelectionDto>,
    pub subtitle_language: Option<String>,
    #[serde(default)]
    pub subtitle_off: bool,
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
            audio: match (r.audio_track, r.audio_language) {
                (Some(index), _) => AudioRequest::Track(index),
                (None, Some(language)) => AudioRequest::Language(LanguageCode(language)),
                (None, None) => AudioRequest::Unspecified,
            },
            subtitle: match (r.subtitle_off, r.subtitle, r.subtitle_language) {
                (true, _, _) => SubtitleRequest::Off,
                (false, Some(selection), _) => SubtitleRequest::Track(selection.into()),
                (false, None, Some(language)) => SubtitleRequest::Language(LanguageCode(language)),
                (false, None, None) => SubtitleRequest::Unspecified,
            },
            target_height: r.target_height,
            force_burn: r.force_burn,
            downmix_stereo: r.downmix_stereo,
        }
    }
}

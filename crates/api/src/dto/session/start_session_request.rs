use serde::Deserialize;

use domain::catalog::VersionId;
use domain::common::LanguageCode;
use domain::session::{AudioRequest, DeliveryPreference, SessionStartInput, SubtitleRequest};

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
    pub delivery: Option<String>,
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
            delivery: r
                .delivery
                .as_deref()
                .map_or(DeliveryPreference::Auto, DeliveryPreference::parse),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(delivery: &str) -> SessionStartInput {
        let json = format!(
            r#"{{"version_id":"v1","capabilities":{{"platform":"chrome","profile_version":1}}{delivery}}}"#
        );
        serde_json::from_str::<StartSessionRequest>(&json)
            .unwrap()
            .into()
    }

    // Every client shipped before this field existed omits it, and must keep
    // getting the negotiated outcome rather than an override.
    #[test]
    fn a_start_without_a_preference_leaves_the_session_to_negotiate() {
        assert_eq!(parse("").delivery, DeliveryPreference::Auto);
    }

    #[test]
    fn a_named_preference_carries_through() {
        assert_eq!(
            parse(r#","delivery":"always""#).delivery,
            DeliveryPreference::AlwaysConvert
        );
        assert_eq!(
            parse(r#","delivery":"nonsense""#).delivery,
            DeliveryPreference::Auto
        );
    }
}

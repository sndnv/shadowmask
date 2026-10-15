use serde::Deserialize;

use domain::session::{DeliveryPreference, SessionUpdate};

use super::SubtitleChangeDto;

#[derive(Debug, Deserialize)]
pub struct UpdateSessionRequest {
    pub position_ms: u64,
    pub audio_track: Option<u32>,
    #[serde(default)]
    pub subtitle: SubtitleChangeDto,
    pub target_height: Option<u32>,
    #[serde(default)]
    pub force_burn: bool,
    #[serde(default)]
    pub downmix_stereo: bool,
    pub delivery: Option<String>,
}

impl From<UpdateSessionRequest> for SessionUpdate {
    fn from(u: UpdateSessionRequest) -> Self {
        SessionUpdate {
            position_ms: u.position_ms,
            audio_track: u.audio_track,
            subtitle: u.subtitle.into(),
            target_height: u.target_height,
            force_burn: u.force_burn,
            downmix_stereo: u.downmix_stereo,
            delivery: u
                .delivery
                .as_deref()
                .map_or(DeliveryPreference::Auto, DeliveryPreference::parse),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> SessionUpdate {
        serde_json::from_str::<UpdateSessionRequest>(json).unwrap().into()
    }

    // Every client shipped before this field existed omits it, and must keep
    // getting the negotiated outcome rather than an override.
    #[test]
    fn a_request_without_a_preference_leaves_the_session_to_negotiate() {
        assert_eq!(parse(r#"{"position_ms":0}"#).delivery, DeliveryPreference::Auto);
    }

    #[test]
    fn a_named_preference_carries_through() {
        assert_eq!(
            parse(r#"{"position_ms":0,"delivery":"never"}"#).delivery,
            DeliveryPreference::NeverConvert
        );
        assert_eq!(
            parse(r#"{"position_ms":0,"delivery":"always"}"#).delivery,
            DeliveryPreference::AlwaysConvert
        );
        assert_eq!(
            parse(r#"{"position_ms":0,"delivery":"nonsense"}"#).delivery,
            DeliveryPreference::Auto
        );
    }

    #[test]
    fn the_position_carries_through() {
        assert_eq!(parse(r#"{"position_ms":61000}"#).position_ms, 61_000);
    }

    #[test]
    fn a_request_without_a_position_is_rejected() {
        assert!(serde_json::from_str::<UpdateSessionRequest>(r#"{"audio_track":1}"#).is_err());
    }
}

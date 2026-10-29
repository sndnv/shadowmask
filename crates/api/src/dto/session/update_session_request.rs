use serde::Deserialize;

use domain::session::{DeliveryPreference, SessionUpdate};

use super::{SoundSettingsDto, SubtitleChangeDto};

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
    #[serde(flatten)]
    pub sound: SoundSettingsDto,
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
            sound: u.sound.into(),
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
    use domain::session::{DialogueLift, NightMode, SoundSettings};

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
    fn an_update_without_sound_settings_normalizes_and_leaves_the_rest_alone() {
        assert_eq!(parse(r#"{"position_ms":0}"#).sound, SoundSettings::default());
    }

    #[test]
    fn the_sound_settings_carry_through() {
        let update =
            parse(r#"{"position_ms":0,"normalize_downmix":false,"dialogue_db":9,"night":"light"}"#);
        assert_eq!(
            update.sound,
            SoundSettings {
                normalize: false,
                dialogue: DialogueLift::Db9,
                night: NightMode::Light
            }
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

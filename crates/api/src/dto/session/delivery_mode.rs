use serde::Serialize;

use domain::session::DeliveryMode;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryModeDto {
    Direct,
    Remux,
    Transcode,
}

impl From<DeliveryMode> for DeliveryModeDto {
    fn from(m: DeliveryMode) -> Self {
        match m {
            DeliveryMode::Direct => DeliveryModeDto::Direct,
            DeliveryMode::Remux => DeliveryModeDto::Remux,
            DeliveryMode::Transcode => DeliveryModeDto::Transcode,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_all_modes() {
        assert!(matches!(DeliveryModeDto::from(DeliveryMode::Direct), DeliveryModeDto::Direct));
        assert!(matches!(DeliveryModeDto::from(DeliveryMode::Remux), DeliveryModeDto::Remux));
        assert!(matches!(
            DeliveryModeDto::from(DeliveryMode::Transcode),
            DeliveryModeDto::Transcode
        ));
    }
}

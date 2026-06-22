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

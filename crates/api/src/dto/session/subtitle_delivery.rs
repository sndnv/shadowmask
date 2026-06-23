use serde::Serialize;

use domain::session::SubtitleDelivery;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubtitleDeliveryDto {
    HlsVtt,
    Burned,
}

impl From<SubtitleDelivery> for SubtitleDeliveryDto {
    fn from(d: SubtitleDelivery) -> Self {
        match d {
            SubtitleDelivery::HlsVtt => SubtitleDeliveryDto::HlsVtt,
            SubtitleDelivery::Burned => SubtitleDeliveryDto::Burned,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_all_deliveries() {
        assert!(matches!(
            SubtitleDeliveryDto::from(SubtitleDelivery::HlsVtt),
            SubtitleDeliveryDto::HlsVtt
        ));
        assert!(matches!(
            SubtitleDeliveryDto::from(SubtitleDelivery::Burned),
            SubtitleDeliveryDto::Burned
        ));
    }
}

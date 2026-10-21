use serde::Serialize;

use domain::session::{AudioDelivery, SubtitleDelivery};

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubtitleDeliveryDto {
    HlsVtt,
    Burned,
    Embedded,
    External,
}

impl From<SubtitleDelivery> for SubtitleDeliveryDto {
    fn from(d: SubtitleDelivery) -> Self {
        match d {
            SubtitleDelivery::HlsVtt => SubtitleDeliveryDto::HlsVtt,
            SubtitleDelivery::Burned => SubtitleDeliveryDto::Burned,
            SubtitleDelivery::Embedded => SubtitleDeliveryDto::Embedded,
            SubtitleDelivery::External => SubtitleDeliveryDto::External,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioDeliveryDto {
    Embedded,
    Stream,
}

impl From<AudioDelivery> for AudioDeliveryDto {
    fn from(d: AudioDelivery) -> Self {
        match d {
            AudioDelivery::Embedded => AudioDeliveryDto::Embedded,
            AudioDelivery::Stream => AudioDeliveryDto::Stream,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wire<T: Serialize>(value: T) -> String {
        serde_json::to_string(&value).unwrap()
    }

    #[test]
    fn maps_all_deliveries() {
        assert_eq!(wire(SubtitleDeliveryDto::from(SubtitleDelivery::HlsVtt)), r#""hls_vtt""#);
        assert_eq!(wire(SubtitleDeliveryDto::from(SubtitleDelivery::Burned)), r#""burned""#);
        assert_eq!(wire(SubtitleDeliveryDto::from(SubtitleDelivery::Embedded)), r#""embedded""#);
        assert_eq!(wire(SubtitleDeliveryDto::from(SubtitleDelivery::External)), r#""external""#);
        assert_eq!(wire(AudioDeliveryDto::from(AudioDelivery::Embedded)), r#""embedded""#);
        assert_eq!(wire(AudioDeliveryDto::from(AudioDelivery::Stream)), r#""stream""#);
    }
}

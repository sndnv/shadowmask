use serde::Serialize;

use domain::session::SelectedTracks;

use super::{AudioDeliveryDto, SubtitleDeliveryDto, SubtitleTrackRefDto};

#[derive(Debug, Serialize)]
pub struct SelectedTracksResponse {
    pub audio_track: Option<u32>,
    pub audio_delivery: Option<AudioDeliveryDto>,
    pub subtitle_track: Option<SubtitleTrackRefDto>,
    pub subtitle_delivery: Option<SubtitleDeliveryDto>,
    pub subtitle_url: Option<String>,
}

impl From<SelectedTracks> for SelectedTracksResponse {
    fn from(s: SelectedTracks) -> Self {
        SelectedTracksResponse {
            audio_track: s.audio_track,
            audio_delivery: s.audio_delivery.map(Into::into),
            subtitle_track: s.subtitle_track.map(Into::into),
            subtitle_delivery: s.subtitle_delivery.map(Into::into),
            subtitle_url: s.subtitle_url,
        }
    }
}

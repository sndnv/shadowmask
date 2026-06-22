use serde::Serialize;

use domain::session::SelectedTracks;

use super::{SubtitleDeliveryDto, SubtitleTrackRefDto};

#[derive(Debug, Serialize)]
pub struct SelectedTracksResponse {
    pub audio_track: Option<u32>,
    pub subtitle_track: Option<SubtitleTrackRefDto>,
    pub subtitle_delivery: Option<SubtitleDeliveryDto>,
}

impl From<SelectedTracks> for SelectedTracksResponse {
    fn from(s: SelectedTracks) -> Self {
        SelectedTracksResponse {
            audio_track: s.audio_track,
            subtitle_track: s.subtitle_track.map(Into::into),
            subtitle_delivery: s.subtitle_delivery.map(Into::into),
        }
    }
}

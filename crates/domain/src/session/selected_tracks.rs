use crate::playback::SubtitleTrackRef;
use crate::session::{AudioDelivery, SubtitleDelivery};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedTracks {
    pub audio_track: Option<u32>,
    pub audio_delivery: Option<AudioDelivery>,
    pub subtitle_track: Option<SubtitleTrackRef>,
    pub subtitle_delivery: Option<SubtitleDelivery>,
    pub subtitle_url: Option<String>,
}

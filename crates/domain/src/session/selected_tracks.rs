use crate::playback::SubtitleTrackRef;
use crate::session::SubtitleDelivery;

#[derive(Debug, Clone)]
pub struct SelectedTracks {
    pub audio_track: Option<u32>,
    pub subtitle_track: Option<SubtitleTrackRef>,
    pub subtitle_delivery: Option<SubtitleDelivery>,
}

use crate::playback::SubtitleTrackRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleSelection {
    pub track: SubtitleTrackRef,
    pub offset_ms: Option<i64>,
}

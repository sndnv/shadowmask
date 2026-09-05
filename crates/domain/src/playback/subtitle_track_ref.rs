use crate::media::SubtitleFileId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubtitleTrackRef {
    Embedded(u32),
    File(SubtitleFileId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubtitleOverride {
    Off,
    Track(SubtitleTrackRef),
}

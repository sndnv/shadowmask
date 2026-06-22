use serde::{Deserialize, Serialize};

use domain::media::SubtitleFileId;
use domain::playback::SubtitleTrackRef;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum SubtitleTrackRefDto {
    Embedded { index: u32 },
    File { id: String },
}

impl From<SubtitleTrackRef> for SubtitleTrackRefDto {
    fn from(r: SubtitleTrackRef) -> Self {
        match r {
            SubtitleTrackRef::Embedded(index) => SubtitleTrackRefDto::Embedded { index },
            SubtitleTrackRef::File(id) => SubtitleTrackRefDto::File { id: id.0 },
        }
    }
}

impl From<SubtitleTrackRefDto> for SubtitleTrackRef {
    fn from(r: SubtitleTrackRefDto) -> Self {
        match r {
            SubtitleTrackRefDto::Embedded { index } => SubtitleTrackRef::Embedded(index),
            SubtitleTrackRefDto::File { id } => SubtitleTrackRef::File(SubtitleFileId(id)),
        }
    }
}

use serde::Deserialize;

use domain::session::SubtitleSelection;

use super::SubtitleTrackRefDto;

#[derive(Debug, Deserialize)]
pub struct SubtitleSelectionDto {
    pub track: SubtitleTrackRefDto,
    pub offset_ms: Option<i64>,
}

impl From<SubtitleSelectionDto> for SubtitleSelection {
    fn from(s: SubtitleSelectionDto) -> Self {
        SubtitleSelection { track: s.track.into(), offset_ms: s.offset_ms }
    }
}

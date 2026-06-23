use serde::Deserialize;

use domain::session::SubtitleChange;

use super::SubtitleSelectionDto;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum SubtitleChangeDto {
    #[default]
    Keep,
    Disable,
    Set(SubtitleSelectionDto),
}

impl From<SubtitleChangeDto> for SubtitleChange {
    fn from(c: SubtitleChangeDto) -> Self {
        match c {
            SubtitleChangeDto::Keep => SubtitleChange::Keep,
            SubtitleChangeDto::Disable => SubtitleChange::Disable,
            SubtitleChangeDto::Set(s) => SubtitleChange::Set(s.into()),
        }
    }
}

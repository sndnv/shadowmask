use crate::common::LanguageCode;
use crate::session::SubtitleSelection;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum AudioRequest {
    #[default]
    Unspecified,
    Track(u32),
    Language(LanguageCode),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SubtitleRequest {
    #[default]
    Unspecified,
    Off,
    Track(SubtitleSelection),
    Language(LanguageCode),
}

use crate::common::LanguageCode;
use crate::media::SubtitleFormat;

#[derive(Debug, Clone)]
pub struct EmbeddedSubtitleTrack {
    pub index: u32,
    pub language: Option<LanguageCode>,
    pub format: SubtitleFormat,
    pub forced: bool,
    pub default: bool,
}

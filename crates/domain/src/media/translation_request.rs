use crate::common::LanguageCode;
use crate::media::SubtitleFormat;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TranslationRequest {
    pub content: String,
    pub format: SubtitleFormat,
    pub source_language: Option<LanguageCode>,
    pub target_language: LanguageCode,
}

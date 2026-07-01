use crate::common::LanguageCode;
use crate::media::SubtitleFormat;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubtitleCandidate {
    pub file_id: String,
    pub language: Option<LanguageCode>,
    pub format: SubtitleFormat,
    pub release_name: Option<String>,
}

use crate::common::LanguageCode;
use crate::media::SubtitleFormat;

#[derive(Debug, Clone, PartialEq)]
pub struct SubtitleCandidate {
    pub file_id: String,
    pub language: Option<LanguageCode>,
    pub format: SubtitleFormat,
    pub release_name: Option<String>,
    pub download_count: Option<u32>,
    pub rating: Option<f32>,
}

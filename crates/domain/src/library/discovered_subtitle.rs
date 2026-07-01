use crate::common::LanguageCode;
use crate::media::SubtitleFormat;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiscoveredSubtitle {
    pub path: String,
    pub language: Option<LanguageCode>,
    pub format: SubtitleFormat,
}

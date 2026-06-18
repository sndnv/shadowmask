use crate::catalog::VersionId;
use crate::common::LanguageCode;
use crate::media::SubtitleFormat;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubtitleFileId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubtitleSource {
    OpenSubtitles,
    External,
}

#[derive(Debug, Clone)]
pub struct SubtitleFile {
    pub id: SubtitleFileId,
    pub version: VersionId,
    pub language: Option<LanguageCode>,
    pub format: SubtitleFormat,
    pub source: SubtitleSource,
    pub path: String,
}

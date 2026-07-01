use crate::media::SubtitleFormat;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedSubtitle {
    pub content: String,
    pub format: SubtitleFormat,
}

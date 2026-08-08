use domain::media::SubtitleCandidate;
use serde::Serialize;

use crate::dto::catalog::SubtitleFormatDto;

#[derive(Debug, Serialize)]
pub struct SubtitleCandidateDto {
    pub file_id: String,
    pub language: Option<String>,
    pub release_name: Option<String>,
    pub format: SubtitleFormatDto,
    pub download_count: Option<u32>,
    pub rating: Option<f32>,
}

impl From<SubtitleCandidate> for SubtitleCandidateDto {
    fn from(candidate: SubtitleCandidate) -> Self {
        Self {
            file_id: candidate.file_id,
            language: candidate.language.map(|code| code.0),
            release_name: candidate.release_name,
            format: candidate.format.into(),
            download_count: candidate.download_count,
            rating: candidate.rating,
        }
    }
}

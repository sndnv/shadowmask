use serde::Serialize;

use domain::library::{MatchCandidate, UnmatchedFile};

use crate::dto::common::TitleRefDto;

#[derive(Debug, Serialize)]
pub struct MatchCandidateDto {
    pub title: TitleRefDto,
    pub confidence: f32,
    pub label: String,
}

impl From<MatchCandidate> for MatchCandidateDto {
    fn from(c: MatchCandidate) -> Self {
        MatchCandidateDto {
            title: c.title.into(),
            confidence: c.confidence,
            label: c.label,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UnmatchedFileResponse {
    pub id: String,
    pub library_id: String,
    pub path: String,
    pub candidates: Vec<MatchCandidateDto>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<UnmatchedFile> for UnmatchedFileResponse {
    fn from(f: UnmatchedFile) -> Self {
        UnmatchedFileResponse {
            id: f.id.0,
            library_id: f.library.0,
            path: f.path,
            candidates: f.candidates.into_iter().map(Into::into).collect(),
            created_at: f.created_at.to_string(),
            updated_at: f.updated_at.to_string(),
        }
    }
}

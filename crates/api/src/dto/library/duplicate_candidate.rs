use serde::Serialize;

use domain::library::DuplicateCandidate;

use crate::dto::common::TitleRefDto;

#[derive(Debug, Serialize)]
pub struct DuplicateCandidateResponse {
    pub id: String,
    pub title: TitleRefDto,
    pub paths: Vec<String>,
}

impl From<DuplicateCandidate> for DuplicateCandidateResponse {
    fn from(d: DuplicateCandidate) -> Self {
        DuplicateCandidateResponse { id: d.id.0, title: d.title.into(), paths: d.paths }
    }
}

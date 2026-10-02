use serde::Serialize;

use domain::library::{DuplicateCandidate, DuplicateKind};

use crate::dto::common::TitleRefDto;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateKindDto {
    Duplicate,
    MultiPart,
}

#[derive(Debug, Serialize)]
pub struct DuplicateCandidateResponse {
    pub id: String,
    pub title: TitleRefDto,
    pub kind: DuplicateKindDto,
    pub paths: Vec<String>,
}

impl From<DuplicateCandidate> for DuplicateCandidateResponse {
    fn from(d: DuplicateCandidate) -> Self {
        let kind = match d.kind() {
            DuplicateKind::Duplicate => DuplicateKindDto::Duplicate,
            DuplicateKind::MultiPart => DuplicateKindDto::MultiPart,
        };
        DuplicateCandidateResponse { id: d.id.0, title: d.title.into(), kind, paths: d.paths }
    }
}

use crate::catalog::TitleId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DuplicateCandidateId(pub String);

#[derive(Debug, Clone)]
pub struct DuplicateCandidate {
    pub id: DuplicateCandidateId,
    pub title: TitleId,
    pub paths: Vec<String>,
}

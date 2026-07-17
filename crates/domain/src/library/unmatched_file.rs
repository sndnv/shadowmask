use jiff::Timestamp;

use crate::catalog::TitleId;
use crate::library::LibraryId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnmatchedFileId(pub String);

#[derive(Debug, Clone)]
pub struct MatchCandidate {
    pub title: TitleId,
    pub confidence: f32,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct UnmatchedFile {
    pub id: UnmatchedFileId,
    pub library: LibraryId,
    pub path: String,
    pub candidates: Vec<MatchCandidate>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

use jiff::Timestamp;

use crate::catalog::TitleId;
use crate::common::Quality;
use crate::library::LibraryId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VersionId(pub String);

#[derive(Debug, Clone)]
pub struct Version {
    pub id: VersionId,
    pub title: TitleId,
    pub library: LibraryId,
    pub quality: Quality,
    pub container: String,
    pub path: String,
    pub size_bytes: u64,
    pub duration_ms: u64,
    pub edition: Option<String>,
    pub available: bool,
    pub added_at: Timestamp,
    pub updated_at: Timestamp,
}

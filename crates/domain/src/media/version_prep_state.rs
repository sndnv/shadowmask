use crate::catalog::VersionId;
use crate::library::LibraryId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionPrepState {
    pub version: VersionId,
    pub library: LibraryId,
    pub prepared_with: Option<String>,
}

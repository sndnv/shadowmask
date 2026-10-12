use domain::diagnostics::LibraryPrep;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LibraryPrepStatus {
    pub id: String,
    pub name: String,
    pub outdated: u64,
    pub unprepared: u64,
    pub prepared_with: Vec<String>,
}

impl From<LibraryPrep> for LibraryPrepStatus {
    fn from(prep: LibraryPrep) -> Self {
        Self {
            id: prep.library.0,
            name: prep.name,
            outdated: prep.outdated,
            unprepared: prep.unprepared,
            prepared_with: prep.prepared_with,
        }
    }
}

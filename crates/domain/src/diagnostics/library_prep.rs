use crate::library::LibraryId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryPrep {
    pub library: LibraryId,
    pub name: String,
    pub outdated: u64,
    pub unprepared: u64,
    pub prepared_with: Vec<String>,
}

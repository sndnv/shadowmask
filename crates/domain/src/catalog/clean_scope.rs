use crate::library::LibraryId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleanScope {
    Library(LibraryId),
    All,
}

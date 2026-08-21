use crate::library::{LibraryKind, LibraryOrigin, WatcherStrategy};

#[derive(Debug, Clone)]
pub struct NewLibrary {
    pub name: String,
    pub kind: LibraryKind,
    pub origin: LibraryOrigin,
    pub roots: Vec<String>,
    pub watcher: WatcherStrategy,
    pub scan_schedule: Option<String>,
    pub metadata_sources: Vec<String>,
    pub sort_articles: Vec<String>,
}

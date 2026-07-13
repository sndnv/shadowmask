use crate::library::{LibraryKind, WatcherStrategy};

#[derive(Debug, Clone)]
pub struct LibraryUpdate {
    pub name: String,
    pub kind: LibraryKind,
    pub roots: Vec<String>,
    pub watcher: WatcherStrategy,
    pub scan_schedule: Option<String>,
    pub metadata_sources: Vec<String>,
}

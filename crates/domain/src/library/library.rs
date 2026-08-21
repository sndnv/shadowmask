use jiff::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LibraryId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LibraryKind {
    Movie,
    Tv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WatcherStrategy {
    Local,
    Polling,
    Scheduled,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LibraryOrigin {
    Local,
    External,
}

#[derive(Debug, Clone)]
pub struct Library {
    pub id: LibraryId,
    pub name: String,
    pub kind: LibraryKind,
    pub origin: LibraryOrigin,
    pub roots: Vec<String>,
    pub watcher: WatcherStrategy,
    pub scan_schedule: Option<String>,
    pub metadata_sources: Vec<String>,
    pub sort_articles: Vec<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

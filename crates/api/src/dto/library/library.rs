use serde::Serialize;

use domain::library::{Library, LibraryKind, WatcherStrategy};

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryKindDto {
    Movie,
    Tv,
}

impl From<LibraryKind> for LibraryKindDto {
    fn from(k: LibraryKind) -> Self {
        match k {
            LibraryKind::Movie => LibraryKindDto::Movie,
            LibraryKind::Tv => LibraryKindDto::Tv,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WatcherStrategyDto {
    Local,
    Polling,
    Scheduled,
    Manual,
}

impl From<WatcherStrategy> for WatcherStrategyDto {
    fn from(w: WatcherStrategy) -> Self {
        match w {
            WatcherStrategy::Local => WatcherStrategyDto::Local,
            WatcherStrategy::Polling => WatcherStrategyDto::Polling,
            WatcherStrategy::Scheduled => WatcherStrategyDto::Scheduled,
            WatcherStrategy::Manual => WatcherStrategyDto::Manual,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LibraryResponse {
    pub id: String,
    pub name: String,
    pub kind: LibraryKindDto,
    pub roots: Vec<String>,
    pub watcher: WatcherStrategyDto,
    pub scan_schedule: Option<String>,
    pub metadata_sources: Vec<String>,
}

impl From<Library> for LibraryResponse {
    fn from(l: Library) -> Self {
        LibraryResponse {
            id: l.id.0,
            name: l.name,
            kind: l.kind.into(),
            roots: l.roots,
            watcher: l.watcher.into(),
            scan_schedule: l.scan_schedule,
            metadata_sources: l.metadata_sources,
        }
    }
}

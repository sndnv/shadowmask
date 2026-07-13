use serde::Deserialize;

use domain::library::LibraryUpdate;

use crate::dto::library::{LibraryKindDto, WatcherStrategyDto};

#[derive(Debug, Deserialize)]
pub struct UpdateLibraryRequest {
    pub name: String,
    pub kind: LibraryKindDto,
    #[serde(default)]
    pub roots: Vec<String>,
    pub watcher: WatcherStrategyDto,
    #[serde(default)]
    pub scan_schedule: Option<String>,
    #[serde(default)]
    pub metadata_sources: Vec<String>,
}

impl From<UpdateLibraryRequest> for LibraryUpdate {
    fn from(r: UpdateLibraryRequest) -> Self {
        LibraryUpdate {
            name: r.name,
            kind: r.kind.into(),
            roots: r.roots,
            watcher: r.watcher.into(),
            scan_schedule: r.scan_schedule,
            metadata_sources: r.metadata_sources,
        }
    }
}

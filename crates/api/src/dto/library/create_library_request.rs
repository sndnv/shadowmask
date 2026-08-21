use serde::Deserialize;

use domain::library::NewLibrary;

use crate::dto::library::{LibraryKindDto, LibraryOriginDto, WatcherStrategyDto};

#[derive(Debug, Deserialize)]
pub struct CreateLibraryRequest {
    pub name: String,
    pub kind: LibraryKindDto,
    #[serde(default)]
    pub origin: LibraryOriginDto,
    #[serde(default)]
    pub roots: Vec<String>,
    pub watcher: WatcherStrategyDto,
    #[serde(default)]
    pub scan_schedule: Option<String>,
    #[serde(default)]
    pub metadata_sources: Vec<String>,
    #[serde(default = "default_sort_articles")]
    pub sort_articles: Vec<String>,
}

fn default_sort_articles() -> Vec<String> {
    vec!["the".to_owned(), "a".to_owned(), "an".to_owned()]
}

impl From<CreateLibraryRequest> for NewLibrary {
    fn from(r: CreateLibraryRequest) -> Self {
        NewLibrary {
            name: r.name,
            kind: r.kind.into(),
            origin: r.origin.into(),
            roots: r.roots,
            watcher: r.watcher.into(),
            scan_schedule: r.scan_schedule,
            metadata_sources: r.metadata_sources,
            sort_articles: r.sort_articles,
        }
    }
}

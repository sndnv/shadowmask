use serde::{Deserialize, Serialize};

use domain::library::{Library, LibraryKind, LibraryOrigin, WatcherStrategy};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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

impl From<LibraryKindDto> for LibraryKind {
    fn from(k: LibraryKindDto) -> Self {
        match k {
            LibraryKindDto::Movie => LibraryKind::Movie,
            LibraryKindDto::Tv => LibraryKind::Tv,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryOriginDto {
    #[default]
    Local,
    External,
}

impl From<LibraryOrigin> for LibraryOriginDto {
    fn from(o: LibraryOrigin) -> Self {
        match o {
            LibraryOrigin::Local => LibraryOriginDto::Local,
            LibraryOrigin::External => LibraryOriginDto::External,
        }
    }
}

impl From<LibraryOriginDto> for LibraryOrigin {
    fn from(o: LibraryOriginDto) -> Self {
        match o {
            LibraryOriginDto::Local => LibraryOrigin::Local,
            LibraryOriginDto::External => LibraryOrigin::External,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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

impl From<WatcherStrategyDto> for WatcherStrategy {
    fn from(w: WatcherStrategyDto) -> Self {
        match w {
            WatcherStrategyDto::Local => WatcherStrategy::Local,
            WatcherStrategyDto::Polling => WatcherStrategy::Polling,
            WatcherStrategyDto::Scheduled => WatcherStrategy::Scheduled,
            WatcherStrategyDto::Manual => WatcherStrategy::Manual,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LibraryResponse {
    pub id: String,
    pub name: String,
    pub kind: LibraryKindDto,
    pub origin: LibraryOriginDto,
    pub roots: Vec<String>,
    pub watcher: WatcherStrategyDto,
    pub scan_schedule: Option<String>,
    pub metadata_sources: Vec<String>,
    pub sort_articles: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Library> for LibraryResponse {
    fn from(l: Library) -> Self {
        LibraryResponse {
            id: l.id.0,
            name: l.name,
            kind: l.kind.into(),
            origin: l.origin.into(),
            roots: l.roots,
            watcher: l.watcher.into(),
            scan_schedule: l.scan_schedule,
            metadata_sources: l.metadata_sources,
            sort_articles: l.sort_articles,
            created_at: l.created_at.to_string(),
            updated_at: l.updated_at.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_all_kinds_and_watchers() {
        assert!(matches!(LibraryKindDto::from(LibraryKind::Movie), LibraryKindDto::Movie));
        assert!(matches!(LibraryKindDto::from(LibraryKind::Tv), LibraryKindDto::Tv));
        assert!(matches!(
            WatcherStrategyDto::from(WatcherStrategy::Local),
            WatcherStrategyDto::Local
        ));
        assert!(matches!(
            WatcherStrategyDto::from(WatcherStrategy::Polling),
            WatcherStrategyDto::Polling
        ));
        assert!(matches!(
            WatcherStrategyDto::from(WatcherStrategy::Scheduled),
            WatcherStrategyDto::Scheduled
        ));
        assert!(matches!(
            WatcherStrategyDto::from(WatcherStrategy::Manual),
            WatcherStrategyDto::Manual
        ));
    }

    #[test]
    fn maps_origins_both_ways() {
        assert!(matches!(LibraryOriginDto::from(LibraryOrigin::Local), LibraryOriginDto::Local));
        assert!(matches!(
            LibraryOriginDto::from(LibraryOrigin::External),
            LibraryOriginDto::External
        ));
        assert_eq!(LibraryOrigin::from(LibraryOriginDto::Local), LibraryOrigin::Local);
        assert_eq!(LibraryOrigin::from(LibraryOriginDto::External), LibraryOrigin::External);
        assert!(matches!(LibraryOriginDto::default(), LibraryOriginDto::Local));
    }

    #[test]
    fn maps_dto_kinds_and_watchers_back() {
        assert_eq!(LibraryKind::from(LibraryKindDto::Movie), LibraryKind::Movie);
        assert_eq!(LibraryKind::from(LibraryKindDto::Tv), LibraryKind::Tv);
        assert_eq!(WatcherStrategy::from(WatcherStrategyDto::Local), WatcherStrategy::Local);
        assert_eq!(WatcherStrategy::from(WatcherStrategyDto::Polling), WatcherStrategy::Polling);
        assert_eq!(
            WatcherStrategy::from(WatcherStrategyDto::Scheduled),
            WatcherStrategy::Scheduled
        );
        assert_eq!(WatcherStrategy::from(WatcherStrategyDto::Manual), WatcherStrategy::Manual);
    }
}

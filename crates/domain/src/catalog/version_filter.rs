use std::collections::HashMap;

use crate::catalog::Version;
use crate::library::LibraryId;

#[derive(Debug, Clone, Default)]
pub struct VersionFilter {
    needle: String,
    library_names: HashMap<LibraryId, String>,
}

impl VersionFilter {
    pub fn new(needle: &str, library_names: HashMap<LibraryId, String>) -> Self {
        Self { needle: needle.trim().to_lowercase(), library_names }
    }

    pub fn matches(&self, version: &Version, title_names: &[String]) -> bool {
        let library = self
            .library_names
            .get(&version.library)
            .map_or(version.library.0.as_str(), String::as_str);
        [version.path.as_str(), library, version.quality.slug(), version.container.as_str()]
            .into_iter()
            .chain(title_names.iter().map(String::as_str))
            .any(|field| field.to_lowercase().contains(&self.needle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{MovieId, TitleId, VersionId};
    use crate::common::Quality;
    use jiff::Timestamp;

    fn version() -> Version {
        Version {
            id: VersionId("v1".into()),
            title: TitleId::Movie(MovieId("m1".into())),
            library: LibraryId("lib1".into()),
            quality: Quality::Fhd,
            container: "mkv".into(),
            path: "/media/Neon Harbor (2018)/Neon Harbor.mkv".into(),
            size_bytes: 1,
            duration_ms: 1,
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn filter(needle: &str) -> VersionFilter {
        VersionFilter::new(needle, HashMap::from([(LibraryId("lib1".into()), "Films".into())]))
    }

    #[test]
    fn every_field_is_searched_ignoring_case() {
        let titles = ["Paper Skies".to_owned()];
        for needle in ["neon HARBOR", "paper", "films", "FHD", "mkv"] {
            assert!(filter(needle).matches(&version(), &titles), "{needle}");
        }
        assert!(!filter("drift").matches(&version(), &titles));
    }

    #[test]
    fn the_needle_is_trimmed() {
        assert!(filter("  films ").matches(&version(), &[]));
    }

    #[test]
    fn a_library_with_no_name_is_matched_by_its_id() {
        let unnamed = VersionFilter::new("lib1", HashMap::new());
        assert!(unnamed.matches(&version(), &[]));
    }
}

use std::collections::HashMap;

use crate::catalog::Version;
use crate::library::LibraryId;

#[derive(Debug, Clone, Default)]
pub struct VersionFilter {
    needle: String,
    available: Option<bool>,
    library_names: HashMap<LibraryId, String>,
}

impl VersionFilter {
    pub fn new(
        needle: &str,
        available: Option<bool>,
        library_names: HashMap<LibraryId, String>,
    ) -> Self {
        Self { needle: needle.trim().to_lowercase(), available, library_names }
    }

    pub fn searches_titles(&self) -> bool {
        !self.needle.is_empty()
    }

    pub fn matches(&self, version: &Version, title_names: &[String]) -> bool {
        if self.available.is_some_and(|available| available != version.available) {
            return false;
        }
        if self.needle.is_empty() {
            return true;
        }
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

    fn libraries() -> HashMap<LibraryId, String> {
        HashMap::from([(LibraryId("lib1".into()), "Films".into())])
    }

    fn filter(needle: &str) -> VersionFilter {
        VersionFilter::new(needle, None, libraries())
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
        let unnamed = VersionFilter::new("lib1", None, HashMap::new());
        assert!(unnamed.matches(&version(), &[]));
    }

    #[test]
    fn availability_narrows_with_or_without_a_needle() {
        let gone = Version { available: false, ..version() };
        for needle in ["", "films"] {
            let unavailable = VersionFilter::new(needle, Some(false), libraries());
            assert!(unavailable.matches(&gone, &[]), "{needle}");
            assert!(!unavailable.matches(&version(), &[]), "{needle}");
            let available = VersionFilter::new(needle, Some(true), libraries());
            assert!(available.matches(&version(), &[]), "{needle}");
            assert!(!available.matches(&gone, &[]), "{needle}");
        }
        assert!(filter("").matches(&gone, &[]));
        assert!(!filter("").searches_titles());
        assert!(filter("films").searches_titles());
    }
}

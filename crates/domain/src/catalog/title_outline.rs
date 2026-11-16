use std::collections::HashMap;

use crate::catalog::{Episode, Movie, Season, Series, TitleId};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VersionCounts {
    pub total: u32,
    pub granted: u32,
    pub present: u32,
}

impl VersionCounts {
    pub fn listed(&self) -> bool {
        self.granted > 0 || self.total == 0
    }
}

#[derive(Debug, Clone, Default)]
pub struct TitleOutline {
    pub movies: Vec<Movie>,
    pub series: Vec<Series>,
    pub seasons: Vec<Season>,
    pub episodes: Vec<Episode>,
    pub versions: HashMap<TitleId, VersionCounts>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_title_is_listed_with_a_granted_version_or_none_at_all() {
        let counts = |total, granted| VersionCounts { total, granted, present: 0 };
        assert!(counts(0, 0).listed());
        assert!(counts(2, 1).listed());
        assert!(!counts(1, 0).listed());
    }
}

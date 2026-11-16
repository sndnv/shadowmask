use crate::catalog::{TitleRow, TitleRowKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowAvailability {
    Available,
    Unavailable,
    NoVersion,
}

impl RowAvailability {
    pub fn parse(value: &str) -> Option<RowAvailability> {
        match value {
            "available" => Some(RowAvailability::Available),
            "unavailable" => Some(RowAvailability::Unavailable),
            "no_version" => Some(RowAvailability::NoVersion),
            _ => None,
        }
    }

    pub fn of(row: &TitleRow) -> RowAvailability {
        if row.available {
            RowAvailability::Available
        } else if row.files == 0 {
            RowAvailability::NoVersion
        } else {
            RowAvailability::Unavailable
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TitleRowFilter {
    needle: String,
    kind: Option<TitleRowKind>,
    availability: Option<RowAvailability>,
}

impl TitleRowFilter {
    pub fn new(
        needle: &str,
        kind: Option<TitleRowKind>,
        availability: Option<RowAvailability>,
    ) -> Self {
        Self { needle: needle.trim().to_lowercase(), kind, availability }
    }

    pub fn matches(&self, row: &TitleRow) -> bool {
        if self.kind.is_some_and(|kind| kind != row.kind) {
            return false;
        }
        if self.availability.is_some_and(|wanted| wanted != RowAvailability::of(row)) {
            return false;
        }
        self.needle.is_empty()
            || [row.title.as_deref(), row.series_title.as_deref()]
                .into_iter()
                .flatten()
                .any(|name| name.to_lowercase().contains(&self.needle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::SeriesId;
    use jiff::Timestamp;

    fn row(kind: TitleRowKind, files: u32, available: bool) -> TitleRow {
        TitleRow {
            kind,
            id: "r1".into(),
            series: None,
            season: None,
            title: Some("Paper Skies".into()),
            series_title: None,
            season_number: None,
            episode_number: None,
            year: None,
            files,
            episodes: None,
            seasons: None,
            available,
            readable: available,
            added_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn season() -> TitleRow {
        TitleRow {
            series: Some(SeriesId("s1".into())),
            title: None,
            series_title: Some("Neon Harbor".into()),
            ..row(TitleRowKind::Season, 0, false)
        }
    }

    #[test]
    fn the_needle_searches_the_title_and_the_series_title_ignoring_case() {
        let movie = row(TitleRowKind::Movie, 1, true);
        for needle in ["", "  ", "paper", " SKIES "] {
            assert!(TitleRowFilter::new(needle, None, None).matches(&movie), "{needle}");
        }
        assert!(!TitleRowFilter::new("harbor", None, None).matches(&movie));
        assert!(TitleRowFilter::new("harbor", None, None).matches(&season()));
        assert!(!TitleRowFilter::new("paper", None, None).matches(&season()));
    }

    #[test]
    fn the_kind_narrows_to_one_kind() {
        let movie = row(TitleRowKind::Movie, 1, true);
        assert!(TitleRowFilter::new("", Some(TitleRowKind::Movie), None).matches(&movie));
        assert!(!TitleRowFilter::new("", Some(TitleRowKind::Season), None).matches(&movie));
        assert!(TitleRowFilter::new("", Some(TitleRowKind::Season), None).matches(&season()));
    }

    #[test]
    fn availability_splits_rows_into_available_unavailable_and_no_version() {
        let present = row(TitleRowKind::Movie, 2, true);
        let missing = row(TitleRowKind::Movie, 1, false);
        let none = row(TitleRowKind::Movie, 0, false);
        assert_eq!(RowAvailability::of(&present), RowAvailability::Available);
        assert_eq!(RowAvailability::of(&missing), RowAvailability::Unavailable);
        assert_eq!(RowAvailability::of(&none), RowAvailability::NoVersion);
        let only = |wanted| TitleRowFilter::new("paper", None, Some(wanted));
        assert!(only(RowAvailability::Unavailable).matches(&missing));
        assert!(!only(RowAvailability::Unavailable).matches(&present));
        assert!(!only(RowAvailability::Unavailable).matches(&none));
        assert!(only(RowAvailability::NoVersion).matches(&none));
        assert!(
            !TitleRowFilter::new("harbor", None, Some(RowAvailability::NoVersion)).matches(&none)
        );
    }

    #[test]
    fn availabilities_parse_from_their_names() {
        assert_eq!(RowAvailability::parse("available"), Some(RowAvailability::Available));
        assert_eq!(RowAvailability::parse("unavailable"), Some(RowAvailability::Unavailable));
        assert_eq!(RowAvailability::parse("no_version"), Some(RowAvailability::NoVersion));
        assert_eq!(RowAvailability::parse("all"), None);
    }
}

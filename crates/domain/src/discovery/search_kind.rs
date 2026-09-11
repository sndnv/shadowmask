#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchKind {
    Movie,
    Series,
    Episode,
    Person,
}

impl SearchKind {
    pub fn parse(value: &str) -> Option<SearchKind> {
        match value {
            "movie" => Some(SearchKind::Movie),
            "series" => Some(SearchKind::Series),
            "episode" => Some(SearchKind::Episode),
            "person" => Some(SearchKind::Person),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SearchKind::Movie => "movie",
            SearchKind::Series => "series",
            SearchKind::Episode => "episode",
            SearchKind::Person => "person",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_round_trips_known_values() {
        for kind in [SearchKind::Movie, SearchKind::Series, SearchKind::Episode, SearchKind::Person]
        {
            assert_eq!(SearchKind::parse(kind.as_str()), Some(kind));
        }
    }

    #[test]
    fn parse_rejects_unknown_value() {
        assert_eq!(SearchKind::parse("collection"), None);
        assert_eq!(SearchKind::parse(""), None);
    }
}

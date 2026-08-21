#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TitleKind {
    Movie,
    Series,
}

impl TitleKind {
    pub fn parse(value: &str) -> Option<TitleKind> {
        match value {
            "movie" => Some(TitleKind::Movie),
            "series" => Some(TitleKind::Series),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            TitleKind::Movie => "movie",
            TitleKind::Series => "series",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_round_trips_and_rejects_unknown() {
        for kind in [TitleKind::Movie, TitleKind::Series] {
            assert_eq!(TitleKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(TitleKind::parse("episode"), None);
    }
}

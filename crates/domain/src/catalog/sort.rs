#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TitleSort {
    #[default]
    AddedAt,
    Title,
    Year,
}

impl TitleSort {
    pub fn parse(value: &str) -> Option<TitleSort> {
        match value {
            "added_at" => Some(TitleSort::AddedAt),
            "title" => Some(TitleSort::Title),
            "year" => Some(TitleSort::Year),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            TitleSort::AddedAt => "added_at",
            TitleSort::Title => "title",
            TitleSort::Year => "year",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

impl SortOrder {
    pub fn parse(value: &str) -> Option<SortOrder> {
        match value {
            "asc" => Some(SortOrder::Asc),
            "desc" => Some(SortOrder::Desc),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SortOrder::Asc => "asc",
            SortOrder::Desc => "desc",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_round_trips_and_rejects_unknown() {
        for sort in [TitleSort::AddedAt, TitleSort::Title, TitleSort::Year] {
            assert_eq!(TitleSort::parse(sort.as_str()), Some(sort));
        }
        for order in [SortOrder::Asc, SortOrder::Desc] {
            assert_eq!(SortOrder::parse(order.as_str()), Some(order));
        }
        assert_eq!(TitleSort::parse("bogus"), None);
        assert_eq!(SortOrder::parse("sideways"), None);
    }

    #[test]
    fn defaults_are_added_at_ascending() {
        assert_eq!(TitleSort::default(), TitleSort::AddedAt);
        assert_eq!(SortOrder::default(), SortOrder::Asc);
    }
}

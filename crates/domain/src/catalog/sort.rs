use jiff::Timestamp;

use crate::catalog::{Movie, Series};

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

pub trait SortableTitle {
    fn sort_id(&self) -> &str;
    fn sort_title(&self) -> &str;
    fn sort_year(&self) -> Option<u16>;
    fn sort_added_at(&self) -> Timestamp;
}

impl SortableTitle for Movie {
    fn sort_id(&self) -> &str {
        &self.id.0
    }
    fn sort_title(&self) -> &str {
        &self.title
    }
    fn sort_year(&self) -> Option<u16> {
        self.year
    }
    fn sort_added_at(&self) -> Timestamp {
        self.added_at
    }
}

impl SortableTitle for Series {
    fn sort_id(&self) -> &str {
        &self.id.0
    }
    fn sort_title(&self) -> &str {
        &self.title
    }
    fn sort_year(&self) -> Option<u16> {
        self.year
    }
    fn sort_added_at(&self) -> Timestamp {
        self.added_at
    }
}

pub fn sort_titles<T: SortableTitle>(items: &mut [T], sort: TitleSort, order: SortOrder) {
    items.sort_by(|a, b| {
        let primary = match sort {
            TitleSort::AddedAt => a.sort_added_at().cmp(&b.sort_added_at()),
            TitleSort::Title => a.sort_title().cmp(b.sort_title()),
            TitleSort::Year => a.sort_year().cmp(&b.sort_year()),
        };
        let primary = match order {
            SortOrder::Asc => primary,
            SortOrder::Desc => primary.reverse(),
        };
        primary.then_with(|| a.sort_id().cmp(b.sort_id()))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::MovieId;

    fn movie(id: &str, title: &str, year: Option<u16>, added: i64) -> Movie {
        Movie {
            id: MovieId(id.into()),
            title: title.into(),
            year,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            added_at: Timestamp::from_second(added).unwrap(),
            updated_at: Timestamp::from_second(added).unwrap(),
            artwork: Vec::new(),
        }
    }

    fn ids(items: &[Movie]) -> Vec<&str> {
        items.iter().map(|m| m.id.0.as_str()).collect()
    }

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

    #[test]
    fn sorts_by_each_key_both_orders() {
        let base = vec![
            movie("b", "Charlie", Some(2001), 30),
            movie("a", "Alpha", Some(2020), 10),
            movie("c", "Bravo", None, 20),
        ];

        let mut by_added = base.clone();
        sort_titles(&mut by_added, TitleSort::AddedAt, SortOrder::Asc);
        assert_eq!(ids(&by_added), ["a", "c", "b"]);
        sort_titles(&mut by_added, TitleSort::AddedAt, SortOrder::Desc);
        assert_eq!(ids(&by_added), ["b", "c", "a"]);

        let mut by_title = base.clone();
        sort_titles(&mut by_title, TitleSort::Title, SortOrder::Asc);
        assert_eq!(ids(&by_title), ["a", "c", "b"]);
        sort_titles(&mut by_title, TitleSort::Title, SortOrder::Desc);
        assert_eq!(ids(&by_title), ["b", "c", "a"]);

        let mut by_year = base.clone();
        sort_titles(&mut by_year, TitleSort::Year, SortOrder::Asc);
        assert_eq!(ids(&by_year), ["c", "b", "a"]);
        sort_titles(&mut by_year, TitleSort::Year, SortOrder::Desc);
        assert_eq!(ids(&by_year), ["a", "b", "c"]);
    }

    fn series(id: &str, title: &str, year: Option<u16>, added: i64) -> Series {
        Series {
            id: crate::catalog::SeriesId(id.into()),
            title: title.into(),
            year,
            overview: None,
            content_rating: None,
            added_at: Timestamp::from_second(added).unwrap(),
            updated_at: Timestamp::from_second(added).unwrap(),
            artwork: Vec::new(),
        }
    }

    #[test]
    fn series_sort_uses_title_and_year() {
        let mut items = vec![
            series("b", "Charlie", Some(2001), 30),
            series("a", "Alpha", Some(2020), 10),
            series("c", "Bravo", None, 20),
        ];
        sort_titles(&mut items, TitleSort::Title, SortOrder::Asc);
        let by_title: Vec<&str> = items.iter().map(|s| s.id.0.as_str()).collect();
        assert_eq!(by_title, ["a", "c", "b"]);

        sort_titles(&mut items, TitleSort::Year, SortOrder::Desc);
        let by_year: Vec<&str> = items.iter().map(|s| s.id.0.as_str()).collect();
        assert_eq!(by_year, ["a", "b", "c"]);
    }

    #[test]
    fn id_breaks_ties_ascending_regardless_of_order() {
        let mut items = vec![
            movie("z", "Same", Some(2000), 5),
            movie("a", "Same", Some(2000), 5),
            movie("m", "Same", Some(2000), 5),
        ];
        sort_titles(&mut items, TitleSort::Title, SortOrder::Desc);
        assert_eq!(ids(&items), ["a", "m", "z"]);
    }
}

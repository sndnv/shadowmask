use domain::catalog::{Movie, Series, SortOrder, TitleSort};
use jiff::Timestamp;

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
        &self.sort_title
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
        &self.sort_title
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
    use domain::catalog::{MovieId, SeriesId};
    use domain::text::sort_title;

    fn movie(id: &str, title: &str, year: Option<u16>, added: i64) -> Movie {
        Movie {
            id: MovieId(id.into()),
            title: title.into(),
            sort_title: sort_title(title, &[]),
            year,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::from_second(added).unwrap(),
            updated_at: Timestamp::from_second(added).unwrap(),
            artwork: Vec::new(),
        }
    }

    fn ids(items: &[Movie]) -> Vec<&str> {
        items.iter().map(|m| m.id.0.as_str()).collect()
    }

    fn series(id: &str, title: &str, year: Option<u16>, added: i64) -> Series {
        Series {
            id: SeriesId(id.into()),
            title: title.into(),
            sort_title: sort_title(title, &[]),
            year,
            overview: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::from_second(added).unwrap(),
            updated_at: Timestamp::from_second(added).unwrap(),
            artwork: Vec::new(),
        }
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

        // Series carry the same four sort keys as movies, but only two of them were
        // ever asked for here, so the other two impls ran in no test at all.
        sort_titles(&mut items, TitleSort::AddedAt, SortOrder::Asc);
        let by_added: Vec<&str> = items.iter().map(|s| s.id.0.as_str()).collect();
        assert_eq!(by_added, ["a", "c", "b"]);

        let mut tied = vec![series("z", "Same", Some(2000), 5), series("a", "Same", Some(2000), 5)];
        sort_titles(&mut tied, TitleSort::Title, SortOrder::Desc);
        let by_id: Vec<&str> = tied.iter().map(|s| s.id.0.as_str()).collect();
        assert_eq!(by_id, ["a", "z"], "id breaks a series tie ascending too");
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

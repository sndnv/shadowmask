use std::cmp::Ordering;

use crate::catalog::{SortOrder, TitleRow, TitleRowKind};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TitleRowSort {
    #[default]
    Title,
    Kind,
    Year,
    Versions,
    Added,
}

impl TitleRowSort {
    pub fn parse(value: &str) -> Option<TitleRowSort> {
        match value {
            "title" => Some(TitleRowSort::Title),
            "kind" => Some(TitleRowSort::Kind),
            "year" => Some(TitleRowSort::Year),
            "versions" => Some(TitleRowSort::Versions),
            "added" => Some(TitleRowSort::Added),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TitleRowOrder {
    pub sort: TitleRowSort,
    pub order: SortOrder,
}

fn group_of(row: &TitleRow) -> String {
    match row.kind {
        TitleRowKind::Movie => format!("movie:{}", row.id),
        TitleRowKind::Series => format!("series:{}", row.id),
        TitleRowKind::Season | TitleRowKind::Episode => {
            format!("series:{}", row.series.as_ref().map_or("", |series| series.0.as_str()))
        }
    }
}

fn reversed_groups(rows: Vec<TitleRow>) -> Vec<TitleRow> {
    let mut groups: Vec<(String, Vec<TitleRow>)> = Vec::new();
    for row in rows {
        let group = group_of(&row);
        match groups.last_mut() {
            Some((last, members)) if *last == group => members.push(row),
            _ => groups.push((group, vec![row])),
        }
    }
    groups.into_iter().rev().flat_map(|(_, members)| members).collect()
}

impl TitleRowOrder {
    pub fn apply(&self, mut rows: Vec<TitleRow>) -> Vec<TitleRow> {
        if self.sort == TitleRowSort::Title {
            return match self.order {
                SortOrder::Asc => rows,
                SortOrder::Desc => reversed_groups(rows),
            };
        }
        rows.sort_by(|a, b| self.compare(a, b));
        rows
    }

    fn directed(&self, ordering: Ordering) -> Ordering {
        match self.order {
            SortOrder::Asc => ordering,
            SortOrder::Desc => ordering.reverse(),
        }
    }

    fn compare(&self, a: &TitleRow, b: &TitleRow) -> Ordering {
        match self.sort {
            TitleRowSort::Kind => self.directed(a.kind.cmp(&b.kind)),
            TitleRowSort::Year => match (a.year, b.year) {
                (Some(left), Some(right)) => self.directed(left.cmp(&right)),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            TitleRowSort::Versions => self.directed(a.files.cmp(&b.files)),
            TitleRowSort::Added | TitleRowSort::Title => self.directed(a.added_at.cmp(&b.added_at)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::SeriesId;
    use jiff::Timestamp;

    fn row(kind: TitleRowKind, id: &str, series: Option<&str>) -> TitleRow {
        TitleRow {
            kind,
            id: id.into(),
            series: series.map(|series| SeriesId(series.into())),
            season: None,
            title: Some(id.into()),
            series_title: None,
            season_number: None,
            episode_number: None,
            year: None,
            files: 0,
            episodes: None,
            seasons: None,
            available: false,
            readable: false,
            added_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn grouped() -> Vec<TitleRow> {
        let at = |second| Timestamp::from_second(second).unwrap();
        vec![
            TitleRow {
                year: Some(2001),
                files: 2,
                added_at: at(5),
                ..row(TitleRowKind::Movie, "m-a", None)
            },
            TitleRow {
                year: Some(2010),
                files: 1,
                added_at: at(3),
                ..row(TitleRowKind::Series, "s", None)
            },
            TitleRow { files: 1, added_at: at(4), ..row(TitleRowKind::Season, "se", Some("s")) },
            TitleRow { files: 1, added_at: at(9), ..row(TitleRowKind::Episode, "e1", Some("s")) },
            TitleRow { added_at: at(1), ..row(TitleRowKind::Episode, "e2", Some("s")) },
            TitleRow { year: Some(2001), added_at: at(7), ..row(TitleRowKind::Movie, "m-b", None) },
        ]
    }

    fn ids(rows: &[TitleRow]) -> Vec<&str> {
        rows.iter().map(|row| row.id.as_str()).collect()
    }

    fn sorted(sort: TitleRowSort, order: SortOrder) -> Vec<TitleRow> {
        TitleRowOrder { sort, order }.apply(grouped())
    }

    #[test]
    fn by_title_the_groups_keep_their_order_or_turn_around_whole() {
        assert_eq!(
            ids(&sorted(TitleRowSort::Title, SortOrder::Asc)),
            ["m-a", "s", "se", "e1", "e2", "m-b"]
        );
        assert_eq!(
            ids(&sorted(TitleRowSort::Title, SortOrder::Desc)),
            ["m-b", "s", "se", "e1", "e2", "m-a"]
        );
        let episodes: Vec<TitleRow> = grouped().into_iter().skip(3).collect();
        let reversed =
            TitleRowOrder { sort: TitleRowSort::Title, order: SortOrder::Desc }.apply(episodes);
        assert_eq!(ids(&reversed), ["m-b", "e1", "e2"], "a series' rows stay together without it");
    }

    #[test]
    fn by_kind_movies_come_first_and_ties_keep_the_title_order() {
        assert_eq!(
            ids(&sorted(TitleRowSort::Kind, SortOrder::Asc)),
            ["m-a", "m-b", "s", "se", "e1", "e2"]
        );
        assert_eq!(
            ids(&sorted(TitleRowSort::Kind, SortOrder::Desc)),
            ["e1", "e2", "se", "s", "m-a", "m-b"]
        );
    }

    #[test]
    fn by_year_rows_without_one_come_last_either_way() {
        assert_eq!(
            ids(&sorted(TitleRowSort::Year, SortOrder::Asc)),
            ["m-a", "m-b", "s", "se", "e1", "e2"]
        );
        assert_eq!(
            ids(&sorted(TitleRowSort::Year, SortOrder::Desc)),
            ["s", "m-a", "m-b", "se", "e1", "e2"]
        );
    }

    #[test]
    fn by_versions_and_by_added_date() {
        assert_eq!(
            ids(&sorted(TitleRowSort::Versions, SortOrder::Asc)),
            ["e2", "m-b", "s", "se", "e1", "m-a"]
        );
        assert_eq!(
            ids(&sorted(TitleRowSort::Versions, SortOrder::Desc)),
            ["m-a", "s", "se", "e1", "e2", "m-b"]
        );
        assert_eq!(
            ids(&sorted(TitleRowSort::Added, SortOrder::Asc)),
            ["e2", "s", "se", "m-a", "m-b", "e1"]
        );
        assert_eq!(
            ids(&sorted(TitleRowSort::Added, SortOrder::Desc)),
            ["e1", "m-b", "m-a", "se", "s", "e2"]
        );
    }

    #[test]
    fn sorts_parse_from_their_names() {
        for (name, sort) in [
            ("title", TitleRowSort::Title),
            ("kind", TitleRowSort::Kind),
            ("year", TitleRowSort::Year),
            ("versions", TitleRowSort::Versions),
            ("added", TitleRowSort::Added),
        ] {
            assert_eq!(TitleRowSort::parse(name), Some(sort));
        }
        assert_eq!(TitleRowSort::parse("files"), None);
        assert_eq!(
            TitleRowOrder::default(),
            TitleRowOrder { sort: TitleRowSort::Title, order: SortOrder::Asc }
        );
    }
}

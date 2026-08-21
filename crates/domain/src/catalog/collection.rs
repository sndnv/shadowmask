use std::collections::HashMap;

use jiff::Timestamp;

use crate::catalog::{ArtworkRef, Movie, MovieId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CollectionId(pub String);

#[derive(Debug, Clone)]
pub struct Collection {
    pub id: CollectionId,
    pub name: String,
    pub overview: Option<String>,
    pub movies: Vec<MovieId>,
    pub added_at: Timestamp,
    pub updated_at: Timestamp,
    pub artwork: Vec<ArtworkRef>,
}

pub fn in_year_order(members: &[MovieId], known: &[Movie]) -> Vec<MovieId> {
    let by_id: HashMap<&MovieId, &Movie> = known.iter().map(|m| (&m.id, m)).collect();
    let mut keyed: Vec<(u16, &str, &MovieId)> = members
        .iter()
        .map(|id| {
            let movie = by_id.get(id).copied();
            (
                movie.and_then(|m| m.year).unwrap_or(u16::MAX),
                movie.map_or("", |m| m.sort_title.as_str()),
                id,
            )
        })
        .collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)));
    keyed.into_iter().map(|(_, _, id)| id.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn movie(id: &str, title: &str, year: Option<u16>) -> Movie {
        Movie {
            id: MovieId(id.to_owned()),
            title: title.to_owned(),
            sort_title: title.to_lowercase(),
            year,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    fn ids(members: &[MovieId]) -> Vec<&str> {
        members.iter().map(|m| m.0.as_str()).collect()
    }

    #[test]
    fn scan_order_becomes_release_order() {
        let known = [
            movie("third", "Part Three", Some(2003)),
            movie("first", "Part One", Some(1999)),
            movie("second", "Part Two", Some(2001)),
        ];
        let members = vec![
            MovieId("third".into()),
            MovieId("first".into()),
            MovieId("second".into()),
        ];
        assert_eq!(
            ids(&in_year_order(&members, &known)),
            vec!["first", "second", "third"]
        );
    }

    #[test]
    fn a_shared_year_falls_back_to_the_sort_title() {
        let known = [
            movie("b", "Beta", Some(2010)),
            movie("a", "Alpha", Some(2010)),
        ];
        let members = vec![MovieId("b".into()), MovieId("a".into())];
        assert_eq!(ids(&in_year_order(&members, &known)), vec!["a", "b"]);
    }

    #[test]
    fn an_unknown_year_sorts_last_rather_than_first() {
        let known = [
            movie("undated", "Zeta", None),
            movie("dated", "Alpha", Some(2010)),
        ];
        let members = vec![MovieId("undated".into()), MovieId("dated".into())];
        assert_eq!(
            ids(&in_year_order(&members, &known)),
            vec!["dated", "undated"]
        );
    }

    #[test]
    fn a_member_with_no_movie_record_is_kept_not_dropped() {
        let known = [movie("dated", "Alpha", Some(2010))];
        let members = vec![MovieId("ghost".into()), MovieId("dated".into())];
        assert_eq!(
            ids(&in_year_order(&members, &known)),
            vec!["dated", "ghost"]
        );
    }
}

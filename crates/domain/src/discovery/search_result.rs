use crate::catalog::{Episode, Movie, Series};
use crate::discovery::SearchKind;
use crate::metadata::Person;

#[derive(Debug, Clone)]
pub enum SearchResult {
    Movie(Movie),
    Series(Series),
    Episode(Episode),
    Person(Person),
}

impl SearchResult {
    pub fn kind(&self) -> SearchKind {
        match self {
            SearchResult::Movie(_) => SearchKind::Movie,
            SearchResult::Series(_) => SearchKind::Series,
            SearchResult::Episode(_) => SearchKind::Episode,
            SearchResult::Person(_) => SearchKind::Person,
        }
    }
}

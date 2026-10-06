use crate::catalog::{EpisodeCard, Movie, Series};
use crate::discovery::SearchKind;
use crate::metadata::Person;

#[derive(Debug, Clone)]
pub enum SearchResult<S = Series> {
    Movie(Movie),
    Series(S),
    Episode(Box<EpisodeCard>),
    Person(Person),
}

impl<S> SearchResult<S> {
    pub fn kind(&self) -> SearchKind {
        match self {
            SearchResult::Movie(_) => SearchKind::Movie,
            SearchResult::Series(_) => SearchKind::Series,
            SearchResult::Episode(_) => SearchKind::Episode,
            SearchResult::Person(_) => SearchKind::Person,
        }
    }

    pub fn map_series<T>(self, map: impl FnOnce(S) -> T) -> SearchResult<T> {
        match self {
            SearchResult::Movie(movie) => SearchResult::Movie(movie),
            SearchResult::Series(series) => SearchResult::Series(map(series)),
            SearchResult::Episode(episode) => SearchResult::Episode(episode),
            SearchResult::Person(person) => SearchResult::Person(person),
        }
    }
}

use crate::catalog::{Episode, Movie, Series};
use crate::metadata::Person;

#[derive(Debug, Clone)]
pub enum SearchResult {
    Movie(Movie),
    Series(Series),
    Episode(Episode),
    Person(Person),
}

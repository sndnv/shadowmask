use serde::Serialize;

use domain::discovery::SearchResult;

use super::PersonResponse;
use crate::dto::catalog::{EpisodeResponse, MovieResponse, SeriesResponse};

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum SearchResultResponse {
    Movie(Box<MovieResponse>),
    Series(Box<SeriesResponse>),
    Episode(Box<EpisodeResponse>),
    Person(Box<PersonResponse>),
}

impl From<SearchResult> for SearchResultResponse {
    fn from(r: SearchResult) -> Self {
        match r {
            SearchResult::Movie(m) => SearchResultResponse::Movie(Box::new(m.into())),
            SearchResult::Series(s) => SearchResultResponse::Series(Box::new(s.into())),
            SearchResult::Episode(e) => SearchResultResponse::Episode(Box::new(e.into())),
            SearchResult::Person(p) => SearchResultResponse::Person(Box::new(p.into())),
        }
    }
}

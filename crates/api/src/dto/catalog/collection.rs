use serde::Serialize;

use domain::catalog::Collection;

use crate::dto::common::ArtworkDto;

#[derive(Debug, Serialize)]
pub struct CollectionResponse {
    pub id: String,
    pub name: String,
    pub overview: Option<String>,
    pub movies: Vec<String>,
    pub artwork: ArtworkDto,
}

impl From<Collection> for CollectionResponse {
    fn from(c: Collection) -> Self {
        CollectionResponse {
            id: c.id.0,
            name: c.name,
            overview: c.overview,
            movies: c.movies.into_iter().map(|m| m.0).collect(),
            artwork: ArtworkDto::from_refs(c.artwork),
        }
    }
}

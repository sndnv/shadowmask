use serde::Serialize;

use domain::catalog::{Collection, CollectionDetail};

use crate::dto::catalog::MovieResponse;
use crate::dto::common::ArtworkDto;

#[derive(Debug, Serialize)]
pub struct CollectionResponse {
    pub id: String,
    pub name: String,
    pub overview: Option<String>,
    pub movies: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<MovieResponse>,
    pub added_at: String,
    pub updated_at: String,
    pub artwork: ArtworkDto,
}

impl From<Collection> for CollectionResponse {
    fn from(c: Collection) -> Self {
        CollectionResponse {
            id: c.id.0,
            name: c.name,
            overview: c.overview,
            movies: c.movies.into_iter().map(|m| m.0).collect(),
            items: Vec::new(),
            added_at: c.added_at.to_string(),
            updated_at: c.updated_at.to_string(),
            artwork: ArtworkDto::from_refs(c.artwork),
        }
    }
}

impl From<CollectionDetail> for CollectionResponse {
    fn from(d: CollectionDetail) -> Self {
        let items = d.movies.into_iter().map(MovieResponse::from).collect();
        CollectionResponse {
            items,
            ..CollectionResponse::from(d.collection)
        }
    }
}

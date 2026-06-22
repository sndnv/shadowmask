use serde::Deserialize;

use domain::catalog::{CollectionUpdate, MovieId};

#[derive(Debug, Deserialize)]
pub struct UpdateCollectionRequest {
    pub name: String,
    pub overview: Option<String>,
    #[serde(default)]
    pub movies: Vec<String>,
}

impl From<UpdateCollectionRequest> for CollectionUpdate {
    fn from(r: UpdateCollectionRequest) -> Self {
        CollectionUpdate {
            name: r.name,
            overview: r.overview,
            movies: r.movies.into_iter().map(MovieId).collect(),
        }
    }
}

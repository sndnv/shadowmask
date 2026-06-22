use serde::Deserialize;

use domain::catalog::{MovieId, NewCollection};

#[derive(Debug, Deserialize)]
pub struct CreateCollectionRequest {
    pub name: String,
    pub overview: Option<String>,
    #[serde(default)]
    pub movies: Vec<String>,
}

impl From<CreateCollectionRequest> for NewCollection {
    fn from(r: CreateCollectionRequest) -> Self {
        NewCollection {
            name: r.name,
            overview: r.overview,
            movies: r.movies.into_iter().map(MovieId).collect(),
        }
    }
}

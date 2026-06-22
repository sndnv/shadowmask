use serde::Serialize;

use domain::catalog::Collection;

#[derive(Debug, Serialize)]
pub struct CollectionResponse {
    pub id: String,
    pub name: String,
    pub overview: Option<String>,
    pub movies: Vec<String>,
}

impl From<Collection> for CollectionResponse {
    fn from(c: Collection) -> Self {
        CollectionResponse {
            id: c.id.0,
            name: c.name,
            overview: c.overview,
            movies: c.movies.into_iter().map(|m| m.0).collect(),
        }
    }
}

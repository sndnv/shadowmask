use crate::catalog::{ArtworkRef, MovieId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CollectionId(pub String);

#[derive(Debug, Clone)]
pub struct Collection {
    pub id: CollectionId,
    pub name: String,
    pub overview: Option<String>,
    pub movies: Vec<MovieId>,
    pub artwork: Vec<ArtworkRef>,
}

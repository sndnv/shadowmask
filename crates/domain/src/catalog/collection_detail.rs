use crate::catalog::{Collection, Movie};

#[derive(Debug, Clone)]
pub struct CollectionDetail {
    pub collection: Collection,
    pub movies: Vec<Movie>,
}

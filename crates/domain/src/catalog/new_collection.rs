use crate::catalog::MovieId;

#[derive(Debug, Clone)]
pub struct NewCollection {
    pub name: String,
    pub overview: Option<String>,
    pub movies: Vec<MovieId>,
}

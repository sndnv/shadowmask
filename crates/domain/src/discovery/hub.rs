use crate::catalog::{Movie, Series};

#[derive(Debug, Clone)]
pub enum HubItem {
    Movie(Movie),
    Series(Series),
}

#[derive(Debug, Clone)]
pub struct Hub {
    pub id: String,
    pub title: String,
    pub items: Vec<HubItem>,
}

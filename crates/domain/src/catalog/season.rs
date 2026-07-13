use crate::catalog::{ArtworkRef, SeriesId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SeasonId(pub String);

#[derive(Debug, Clone)]
pub struct Season {
    pub id: SeasonId,
    pub series: SeriesId,
    pub number: u16,
    pub title: Option<String>,
    pub overview: Option<String>,
    pub artwork: Vec<ArtworkRef>,
}

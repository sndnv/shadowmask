use jiff::Timestamp;

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
    pub added_at: Timestamp,
    pub updated_at: Timestamp,
    pub artwork: Vec<ArtworkRef>,
}

use jiff::Timestamp;

use crate::catalog::{ArtworkRef, SeasonId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EpisodeId(pub String);

#[derive(Debug, Clone)]
pub struct Episode {
    pub id: EpisodeId,
    pub season: SeasonId,
    pub number: u16,
    pub title: String,
    pub overview: Option<String>,
    pub runtime_minutes: Option<u32>,
    pub air_date: Option<Timestamp>,
    pub added_at: Timestamp,
    pub updated_at: Timestamp,
    pub artwork: Vec<ArtworkRef>,
}

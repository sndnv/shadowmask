use serde::Serialize;

use domain::catalog::Episode;

use crate::dto::common::ArtworkDto;

#[derive(Debug, Serialize)]
pub struct EpisodeResponse {
    pub id: String,
    pub season_id: String,
    pub number: u16,
    pub title: String,
    pub overview: Option<String>,
    pub runtime_minutes: Option<u32>,
    pub air_date: Option<String>,
    pub added_at: String,
    pub artwork: ArtworkDto,
}

impl From<Episode> for EpisodeResponse {
    fn from(e: Episode) -> Self {
        EpisodeResponse {
            id: e.id.0,
            season_id: e.season.0,
            number: e.number,
            title: e.title,
            overview: e.overview,
            runtime_minutes: e.runtime_minutes,
            air_date: e.air_date.map(|d| d.to_string()),
            added_at: e.added_at.to_string(),
            artwork: ArtworkDto::from_refs(e.artwork),
        }
    }
}

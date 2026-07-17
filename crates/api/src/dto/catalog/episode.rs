use serde::Serialize;

use domain::catalog::{Episode, EpisodeCard};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season_number: Option<u16>,
    pub added_at: String,
    pub updated_at: String,
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
            series_id: None,
            series_title: None,
            season_number: None,
            added_at: e.added_at.to_string(),
            updated_at: e.updated_at.to_string(),
            artwork: ArtworkDto::from_refs(e.artwork),
        }
    }
}

impl From<EpisodeCard> for EpisodeResponse {
    fn from(card: EpisodeCard) -> Self {
        EpisodeResponse {
            series_id: card.series.map(|s| s.0),
            series_title: card.series_title,
            season_number: card.season_number,
            ..EpisodeResponse::from(card.episode)
        }
    }
}

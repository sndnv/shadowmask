use serde::Serialize;

use domain::catalog::{Season, SeasonCard};

use crate::dto::common::ArtworkDto;

#[derive(Debug, Serialize)]
pub struct SeasonResponse {
    pub id: String,
    pub series_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_title: Option<String>,
    #[serde(skip_serializing_if = "ArtworkDto::is_empty")]
    pub series_artwork: ArtworkDto,
    pub number: u16,
    pub title: Option<String>,
    pub overview: Option<String>,
    pub added_at: String,
    pub updated_at: String,
    pub artwork: ArtworkDto,
}

impl From<Season> for SeasonResponse {
    fn from(s: Season) -> Self {
        SeasonResponse {
            id: s.id.0,
            series_id: s.series.0,
            series_title: None,
            series_artwork: ArtworkDto::default(),
            number: s.number,
            title: s.title,
            overview: s.overview,
            added_at: s.added_at.to_string(),
            updated_at: s.updated_at.to_string(),
            artwork: ArtworkDto::from_refs(s.artwork),
        }
    }
}

impl From<SeasonCard> for SeasonResponse {
    fn from(card: SeasonCard) -> Self {
        SeasonResponse {
            series_title: card.series_title,
            series_artwork: ArtworkDto::from_refs(card.series_artwork),
            ..SeasonResponse::from(card.season)
        }
    }
}

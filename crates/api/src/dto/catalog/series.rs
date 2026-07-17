use serde::Serialize;

use domain::catalog::Series;

use crate::dto::common::{ArtworkDto, ContentRatingDto};

#[derive(Debug, Serialize)]
pub struct SeriesResponse {
    pub id: String,
    pub title: String,
    pub year: Option<u16>,
    pub overview: Option<String>,
    pub content_rating: Option<ContentRatingDto>,
    pub added_at: String,
    pub updated_at: String,
    pub artwork: ArtworkDto,
}

impl From<Series> for SeriesResponse {
    fn from(s: Series) -> Self {
        SeriesResponse {
            id: s.id.0,
            title: s.title,
            year: s.year,
            overview: s.overview,
            content_rating: s.content_rating.map(Into::into),
            added_at: s.added_at.to_string(),
            updated_at: s.updated_at.to_string(),
            artwork: ArtworkDto::from_refs(s.artwork),
        }
    }
}

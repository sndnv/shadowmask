use serde::Serialize;

use domain::catalog::Movie;

use crate::dto::common::{ArtworkDto, ContentRatingDto};

#[derive(Debug, Serialize)]
pub struct MovieResponse {
    pub id: String,
    pub title: String,
    pub year: Option<u16>,
    pub overview: Option<String>,
    pub runtime_minutes: Option<u32>,
    pub content_rating: Option<ContentRatingDto>,
    pub manually_edited: bool,
    pub added_at: String,
    pub updated_at: String,
    pub artwork: ArtworkDto,
}

impl From<Movie> for MovieResponse {
    fn from(m: Movie) -> Self {
        MovieResponse {
            id: m.id.0,
            title: m.title,
            year: m.year,
            overview: m.overview,
            runtime_minutes: m.runtime_minutes,
            content_rating: m.content_rating.map(Into::into),
            manually_edited: m.manually_edited,
            added_at: m.added_at.to_string(),
            updated_at: m.updated_at.to_string(),
            artwork: ArtworkDto::from_refs(m.artwork),
        }
    }
}

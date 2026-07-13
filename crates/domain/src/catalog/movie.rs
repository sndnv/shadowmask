use jiff::Timestamp;

use crate::catalog::ArtworkRef;
use crate::metadata::ContentRating;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MovieId(pub String);

#[derive(Debug, Clone)]
pub struct Movie {
    pub id: MovieId,
    pub title: String,
    pub year: Option<u16>,
    pub overview: Option<String>,
    pub runtime_minutes: Option<u32>,
    pub content_rating: Option<ContentRating>,
    pub added_at: Timestamp,
    pub artwork: Vec<ArtworkRef>,
}

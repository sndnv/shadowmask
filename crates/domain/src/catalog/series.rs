use jiff::Timestamp;

use crate::catalog::ArtworkRef;
use crate::metadata::ContentRating;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SeriesId(pub String);

#[derive(Debug, Clone)]
pub struct Series {
    pub id: SeriesId,
    pub title: String,
    pub year: Option<u16>,
    pub overview: Option<String>,
    pub content_rating: Option<ContentRating>,
    pub added_at: Timestamp,
    pub artwork: Vec<ArtworkRef>,
}

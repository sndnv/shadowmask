use crate::metadata::ContentRating;

#[derive(Debug, Clone)]
pub struct SeriesEdit {
    pub title: String,
    pub year: Option<u16>,
    pub overview: Option<String>,
    pub content_rating: Option<ContentRating>,
}

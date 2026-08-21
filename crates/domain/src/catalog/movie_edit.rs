use crate::metadata::ContentRating;

#[derive(Debug, Clone)]
pub struct MovieEdit {
    pub title: String,
    pub year: Option<u16>,
    pub overview: Option<String>,
    pub runtime_minutes: Option<u32>,
    pub content_rating: Option<ContentRating>,
}

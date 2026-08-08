use crate::catalog::{SortOrder, TitleSort};
use crate::library::LibraryId;
use crate::metadata::ContentRating;

#[derive(Debug, Clone, Default)]
pub struct TitleListFilter {
    pub genres: Vec<String>,
    pub libraries: Option<Vec<LibraryId>>,
    pub blocked_ratings: Vec<ContentRating>,
    pub sort: TitleSort,
    pub order: SortOrder,
}

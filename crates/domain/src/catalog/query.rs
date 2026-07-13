use crate::catalog::{SortOrder, TitleSort};
use crate::library::LibraryId;
use crate::metadata::GenreId;

#[derive(Debug, Clone, Default)]
pub struct TitleListQuery {
    pub genre: Option<GenreId>,
    pub library: Option<LibraryId>,
    pub sort: TitleSort,
    pub order: SortOrder,
}

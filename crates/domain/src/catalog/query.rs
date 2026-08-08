use crate::catalog::{SortOrder, TitleSort};
use crate::library::LibraryId;

#[derive(Debug, Clone, Default)]
pub struct TitleListQuery {
    pub genres: Vec<String>,
    pub library: Option<LibraryId>,
    pub sort: TitleSort,
    pub order: SortOrder,
}

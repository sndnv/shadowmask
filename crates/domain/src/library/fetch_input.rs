use crate::library::LibraryKind;

#[derive(Debug, Clone)]
pub struct FetchInput {
    pub source_url: String,
    pub kind: LibraryKind,
    pub title: String,
    pub year: Option<u16>,
    pub external_id: Option<String>,
    pub season: Option<u16>,
    pub episode: Option<u16>,
}

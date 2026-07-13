use crate::catalog::{ArtworkRef, TitleRef};
use crate::metadata::{CreditRole, Person};

#[derive(Debug, Clone)]
pub struct PersonProfile {
    pub person: Person,
    pub filmography: Vec<FilmographyEntry>,
}

#[derive(Debug, Clone)]
pub struct FilmographyEntry {
    pub title: TitleRef,
    pub display_title: String,
    pub year: Option<u16>,
    pub artwork: Vec<ArtworkRef>,
    pub role: CreditRole,
    pub character: Option<String>,
}

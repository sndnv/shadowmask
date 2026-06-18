use crate::common::LanguageCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtworkKind {
    Poster,
    Backdrop,
    Banner,
    Logo,
    ClearArt,
}

#[derive(Debug, Clone)]
pub struct Artwork {
    pub kind: ArtworkKind,
    pub language: Option<LanguageCode>,
    pub source: String,
    pub url: String,
}

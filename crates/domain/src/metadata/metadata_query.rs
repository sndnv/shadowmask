use crate::metadata::MediaKind;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MetadataQuery {
    pub title: String,
    pub year: Option<u16>,
    pub kind: MediaKind,
}

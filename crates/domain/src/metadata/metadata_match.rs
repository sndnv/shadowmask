use crate::metadata::{ExternalId, MediaKind};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MetadataMatch {
    pub external_id: ExternalId,
    pub title: String,
    pub year: Option<u16>,
    pub kind: MediaKind,
}

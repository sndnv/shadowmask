use crate::catalog::ArtworkId;
use crate::metadata::ArtworkKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtworkRef {
    pub id: ArtworkId,
    pub kind: ArtworkKind,
    pub widths: Vec<u32>,
}

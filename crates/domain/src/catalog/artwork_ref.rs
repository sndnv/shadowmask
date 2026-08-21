use crate::catalog::ArtworkId;
use crate::metadata::ArtworkKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtworkWidth {
    pub width: u32,
    pub path: String,
}

impl ArtworkWidth {
    pub fn new(width: u32, path: impl Into<String>) -> Self {
        Self {
            width,
            path: path.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtworkRef {
    pub id: ArtworkId,
    pub kind: ArtworkKind,
    pub widths: Vec<ArtworkWidth>,
}

impl ArtworkRef {
    pub fn sizes(&self) -> Vec<u32> {
        self.widths.iter().map(|width| width.width).collect()
    }
}

use crate::catalog::VersionId;

#[derive(Debug, Clone)]
pub struct Chapter {
    pub version: VersionId,
    pub title: String,
    pub start_ms: u64,
}

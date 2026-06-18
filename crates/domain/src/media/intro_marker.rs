use crate::catalog::VersionId;

#[derive(Debug, Clone)]
pub struct IntroMarker {
    pub version: VersionId,
    pub start_ms: u64,
    pub end_ms: u64,
}

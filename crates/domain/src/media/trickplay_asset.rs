use crate::catalog::VersionId;

#[derive(Debug, Clone)]
pub struct TrickplayAsset {
    pub version: VersionId,
    pub interval_ms: u64,
    pub columns: u32,
    pub rows: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub sheet_paths: Vec<String>,
}

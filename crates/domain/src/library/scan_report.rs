use crate::library::{DiscoveredFile, SkippedFile};

#[derive(Debug, Clone, Default)]
pub struct ScanReport {
    pub discovered: Vec<DiscoveredFile>,
    pub skipped: Vec<SkippedFile>,
    pub total_candidates: usize,
}

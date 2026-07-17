use crate::library::LibraryId;
use crate::media::ProbeResult;

#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    pub library: LibraryId,
    pub path: String,
    pub size_bytes: u64,
    pub subtitle_siblings: Vec<String>,
    pub probe: ProbeResult,
}

use crate::library::{DiscoveredFile, MatchKey, ParsedMedia};

#[derive(Debug, Clone)]
pub struct MatchedGroup {
    pub key: MatchKey,
    pub parsed: ParsedMedia,
    pub confidence: f32,
    pub files: Vec<DiscoveredFile>,
}

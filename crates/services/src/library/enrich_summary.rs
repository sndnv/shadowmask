use std::collections::HashSet;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnrichSummary {
    pub matched: usize,
    pub titles: usize,
    pub unmatched: HashSet<String>,
    pub duplicates: usize,
    pub failed: usize,
}

use crate::library::{MatchedGroup, UnmatchedFile};

#[derive(Debug, Clone, Default)]
pub struct MatchReport {
    pub matched: Vec<MatchedGroup>,
    pub unmatched: Vec<UnmatchedFile>,
}

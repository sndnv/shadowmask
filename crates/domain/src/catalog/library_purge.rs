use std::collections::HashSet;

use crate::catalog::{TitleSet, VersionId};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LibraryPurge {
    pub titles: TitleSet,
    pub versions: HashSet<VersionId>,
    pub subtitles: u64,
}

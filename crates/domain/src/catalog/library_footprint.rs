use std::collections::HashSet;

use crate::catalog::{TitleSet, VersionId};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LibraryFootprint {
    pub versions: HashSet<VersionId>,
    pub titles: TitleSet,
}

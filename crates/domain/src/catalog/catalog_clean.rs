use std::collections::HashSet;

use crate::catalog::{TitleSet, VersionId};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CatalogClean {
    pub titles: TitleSet,
    pub versions: HashSet<VersionId>,
}

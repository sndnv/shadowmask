use std::collections::HashSet;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VersionReconcile {
    pub known: HashSet<String>,
    pub gone: usize,
    pub returned: usize,
}

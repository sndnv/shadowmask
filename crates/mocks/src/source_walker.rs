use std::collections::{HashMap, HashSet};

use domain::error::WalkError;
use domain::library::{SourceWalker, WalkedEntry};

#[derive(Clone, Default)]
pub struct MockSourceWalker {
    by_root: HashMap<String, Vec<WalkedEntry>>,
    not_found: HashSet<String>,
    unreadable: HashSet<String>,
}

impl MockSourceWalker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_entries(mut self, root: &str, entries: Vec<WalkedEntry>) -> Self {
        self.by_root.insert(root.to_owned(), entries);
        self
    }

    pub fn with_failing(mut self, root: &str) -> Self {
        self.not_found.insert(root.to_owned());
        self
    }

    pub fn with_unreadable(mut self, root: &str) -> Self {
        self.unreadable.insert(root.to_owned());
        self
    }
}

impl SourceWalker for MockSourceWalker {
    async fn walk(&self, root: &str) -> Result<Vec<WalkedEntry>, WalkError> {
        if self.not_found.contains(root) {
            return Err(WalkError::RootNotFound(root.to_owned()));
        }
        if self.unreadable.contains(root) {
            return Err(WalkError::Unreadable(root.to_owned()));
        }
        Ok(self.by_root.get(root).cloned().unwrap_or_default())
    }
}

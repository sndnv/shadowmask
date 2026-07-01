use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use domain::error::RepositoryError;
use domain::library::{DuplicateCandidate, Library, LibraryId, ScanState, UnmatchedFile};
use domain::repository::LibraryRepository;

#[derive(Clone, Default)]
pub struct MockLibraryRepo {
    libraries: Arc<Mutex<HashMap<LibraryId, Library>>>,
    scan_states: Arc<Mutex<HashMap<LibraryId, ScanState>>>,
    fail_get: Arc<AtomicBool>,
    fail_save: Arc<AtomicBool>,
}

impl MockLibraryRepo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_library(&self, library: Library) {
        self.libraries
            .lock()
            .unwrap()
            .insert(library.id.clone(), library);
    }

    pub fn set_fail_get(&self) {
        self.fail_get.store(true, Ordering::Relaxed);
    }

    pub fn set_fail_save(&self) {
        self.fail_save.store(true, Ordering::Relaxed);
    }
}

impl LibraryRepository for MockLibraryRepo {
    async fn list(&self) -> Result<Vec<Library>, RepositoryError> {
        Ok(self.libraries.lock().unwrap().values().cloned().collect())
    }

    async fn get(&self, id: &LibraryId) -> Result<Option<Library>, RepositoryError> {
        if self.fail_get.load(Ordering::Relaxed) {
            return Err(RepositoryError::Backend("mock get failure".to_owned()));
        }
        Ok(self.libraries.lock().unwrap().get(id).cloned())
    }

    async fn scan_state(&self, id: &LibraryId) -> Result<Option<ScanState>, RepositoryError> {
        Ok(self.scan_states.lock().unwrap().get(id).cloned())
    }

    async fn save_scan_state(&self, state: ScanState) -> Result<(), RepositoryError> {
        if self.fail_save.load(Ordering::Relaxed) {
            return Err(RepositoryError::Backend("mock save failure".to_owned()));
        }
        self.scan_states
            .lock()
            .unwrap()
            .insert(state.library.clone(), state);
        Ok(())
    }

    async fn list_unmatched(&self, _id: &LibraryId) -> Result<Vec<UnmatchedFile>, RepositoryError> {
        Ok(Vec::new())
    }

    async fn list_duplicates(
        &self,
        _id: &LibraryId,
    ) -> Result<Vec<DuplicateCandidate>, RepositoryError> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::library::{LibraryKind, WatcherStrategy};

    fn library() -> Library {
        Library {
            id: LibraryId("l".into()),
            name: "n".into(),
            kind: LibraryKind::Movie,
            roots: Vec::new(),
            watcher: WatcherStrategy::Manual,
            scan_schedule: None,
            metadata_sources: Vec::new(),
        }
    }

    #[tokio::test]
    async fn list_get_and_empty_queues() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(library());
        let id = LibraryId("l".into());

        assert_eq!(repo.list().await.unwrap().len(), 1);
        assert!(repo.get(&id).await.unwrap().is_some());
        assert!(repo.scan_state(&id).await.unwrap().is_none());
        assert!(repo.list_unmatched(&id).await.unwrap().is_empty());
        assert!(repo.list_duplicates(&id).await.unwrap().is_empty());
    }
}

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use domain::common::{Page, PageRequest};
use domain::error::RepositoryError;
use domain::library::{
    DuplicateCandidate, DuplicateCandidateId, Library, LibraryId, ResolutionStatus, ScanState,
    UnmatchedFile, UnmatchedFileId,
};
use domain::repository::LibraryRepository;

use crate::page::paginate;

type UnmatchedEntry = (UnmatchedFile, ResolutionStatus);
type DuplicateEntry = (LibraryId, DuplicateCandidate, ResolutionStatus);

#[derive(Clone, Default)]
pub struct MockLibraryRepo {
    libraries: Arc<Mutex<HashMap<LibraryId, Library>>>,
    scan_states: Arc<Mutex<HashMap<LibraryId, ScanState>>>,
    unmatched: Arc<Mutex<HashMap<UnmatchedFileId, UnmatchedEntry>>>,
    duplicates: Arc<Mutex<HashMap<DuplicateCandidateId, DuplicateEntry>>>,
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

    async fn upsert(&self, library: Library) -> Result<(), RepositoryError> {
        if self.fail_save.load(Ordering::Relaxed) {
            return Err(RepositoryError::Backend("mock save failure".to_owned()));
        }
        let mut map = self.libraries.lock().unwrap();
        let mut library = library;
        if let Some(existing) = map.get(&library.id) {
            library.created_at = existing.created_at;
        }
        map.insert(library.id.clone(), library);
        Ok(())
    }

    async fn delete(&self, id: &LibraryId) -> Result<(), RepositoryError> {
        if self.fail_save.load(Ordering::Relaxed) {
            return Err(RepositoryError::Backend("mock save failure".to_owned()));
        }
        self.libraries.lock().unwrap().remove(id);
        self.scan_states.lock().unwrap().remove(id);
        self.unmatched
            .lock()
            .unwrap()
            .retain(|_, (file, _)| file.library != *id);
        self.duplicates
            .lock()
            .unwrap()
            .retain(|_, (library, _, _)| library != id);
        Ok(())
    }

    async fn list_unmatched(
        &self,
        id: &LibraryId,
        page: PageRequest,
    ) -> Result<Page<UnmatchedFile>, RepositoryError> {
        let mut items: Vec<UnmatchedFile> = self
            .unmatched
            .lock()
            .unwrap()
            .values()
            .filter(|(file, status)| file.library == *id && *status == ResolutionStatus::Active)
            .map(|(file, _)| file.clone())
            .collect();
        items.sort_by(|a, b| a.id.0.cmp(&b.id.0));
        Ok(paginate(&items, page))
    }

    async fn list_duplicates(
        &self,
        id: &LibraryId,
        page: PageRequest,
    ) -> Result<Page<DuplicateCandidate>, RepositoryError> {
        let mut items: Vec<DuplicateCandidate> = self
            .duplicates
            .lock()
            .unwrap()
            .values()
            .filter(|(library, _, status)| library == id && *status == ResolutionStatus::Active)
            .map(|(_, duplicate, _)| duplicate.clone())
            .collect();
        items.sort_by(|a, b| a.id.0.cmp(&b.id.0));
        Ok(paginate(&items, page))
    }

    async fn get_unmatched(
        &self,
        id: &UnmatchedFileId,
    ) -> Result<Option<UnmatchedFile>, RepositoryError> {
        if self.fail_get.load(Ordering::Relaxed) {
            return Err(RepositoryError::Backend("mock get failure".to_owned()));
        }
        Ok(self
            .unmatched
            .lock()
            .unwrap()
            .get(id)
            .map(|(file, _)| file.clone()))
    }

    async fn insert_unmatched(&self, file: UnmatchedFile) -> Result<(), RepositoryError> {
        if self.fail_save.load(Ordering::Relaxed) {
            return Err(RepositoryError::Backend("mock save failure".to_owned()));
        }
        let mut map = self.unmatched.lock().unwrap();
        let mut file = file;
        let status = match map.get(&file.id) {
            Some((existing, status)) => {
                file.created_at = existing.created_at;
                *status
            }
            None => ResolutionStatus::Active,
        };
        map.insert(file.id.clone(), (file, status));
        Ok(())
    }

    async fn insert_duplicate(
        &self,
        library: &LibraryId,
        duplicate: DuplicateCandidate,
    ) -> Result<(), RepositoryError> {
        if self.fail_save.load(Ordering::Relaxed) {
            return Err(RepositoryError::Backend("mock save failure".to_owned()));
        }
        let mut map = self.duplicates.lock().unwrap();
        let status = map
            .get(&duplicate.id)
            .map(|(_, _, status)| *status)
            .unwrap_or(ResolutionStatus::Active);
        map.insert(duplicate.id.clone(), (library.clone(), duplicate, status));
        Ok(())
    }

    async fn set_unmatched_status(
        &self,
        id: &UnmatchedFileId,
        status: ResolutionStatus,
    ) -> Result<(), RepositoryError> {
        if let Some(entry) = self.unmatched.lock().unwrap().get_mut(id) {
            entry.1 = status;
        }
        Ok(())
    }

    async fn set_duplicate_status(
        &self,
        id: &DuplicateCandidateId,
        status: ResolutionStatus,
    ) -> Result<(), RepositoryError> {
        if let Some(entry) = self.duplicates.lock().unwrap().get_mut(id) {
            entry.2 = status;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::library::{LibraryKind, WatcherStrategy};
    use jiff::Timestamp;

    fn library() -> Library {
        Library {
            id: LibraryId("l".into()),
            name: "n".into(),
            kind: LibraryKind::Movie,
            roots: Vec::new(),
            watcher: WatcherStrategy::Manual,
            scan_schedule: None,
            metadata_sources: Vec::new(),
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn page() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 10,
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
        assert!(
            repo.list_unmatched(&id, page())
                .await
                .unwrap()
                .items
                .is_empty()
        );
        assert!(
            repo.list_duplicates(&id, page())
                .await
                .unwrap()
                .items
                .is_empty()
        );
    }
}

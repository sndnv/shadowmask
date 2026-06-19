use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use domain::error::LibraryError;
use domain::library::{
    DuplicateCandidate, Library, LibraryId, ScanState, ScanStatus, UnmatchedFile,
};
use domain::service::LibraryService;

#[derive(Debug, Default)]
struct State {
    libraries: Vec<Library>,
    scans: HashMap<LibraryId, ScanState>,
    unmatched: HashMap<LibraryId, Vec<UnmatchedFile>>,
    duplicates: HashMap<LibraryId, Vec<DuplicateCandidate>>,
}

#[derive(Clone, Default)]
pub struct MockLibraryService {
    state: Arc<Mutex<State>>,
}

impl MockLibraryService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_library(&self, library: Library) {
        self.state.lock().unwrap().libraries.push(library);
    }

    pub fn add_unmatched(&self, id: &LibraryId, file: UnmatchedFile) {
        self.state
            .lock()
            .unwrap()
            .unmatched
            .entry(id.clone())
            .or_default()
            .push(file);
    }

    pub fn add_duplicate(&self, id: &LibraryId, candidate: DuplicateCandidate) {
        self.state
            .lock()
            .unwrap()
            .duplicates
            .entry(id.clone())
            .or_default()
            .push(candidate);
    }
}

fn require_library(state: &State, id: &LibraryId) -> Result<(), LibraryError> {
    if state.libraries.iter().any(|l| &l.id == id) {
        Ok(())
    } else {
        Err(LibraryError::NotFound)
    }
}

impl LibraryService for MockLibraryService {
    async fn libraries(&self) -> Result<Vec<Library>, LibraryError> {
        Ok(self.state.lock().unwrap().libraries.clone())
    }

    async fn library(&self, id: &LibraryId) -> Result<Library, LibraryError> {
        self.state
            .lock()
            .unwrap()
            .libraries
            .iter()
            .find(|l| &l.id == id)
            .cloned()
            .ok_or(LibraryError::NotFound)
    }

    async fn scan_state(&self, id: &LibraryId) -> Result<ScanState, LibraryError> {
        let state = self.state.lock().unwrap();
        require_library(&state, id)?;
        Ok(state.scans.get(id).cloned().unwrap_or(ScanState {
            library: id.clone(),
            status: ScanStatus::Idle,
            progress: 0.0,
            last_scanned_at: None,
            error: None,
        }))
    }

    async fn trigger_scan(&self, id: &LibraryId) -> Result<(), LibraryError> {
        let mut state = self.state.lock().unwrap();
        require_library(&state, id)?;
        if matches!(state.scans.get(id), Some(s) if s.status == ScanStatus::Running) {
            return Err(LibraryError::ScanInProgress);
        }
        state.scans.insert(
            id.clone(),
            ScanState {
                library: id.clone(),
                status: ScanStatus::Running,
                progress: 0.0,
                last_scanned_at: None,
                error: None,
            },
        );
        Ok(())
    }

    async fn unmatched(&self, id: &LibraryId) -> Result<Vec<UnmatchedFile>, LibraryError> {
        let state = self.state.lock().unwrap();
        require_library(&state, id)?;
        Ok(state.unmatched.get(id).cloned().unwrap_or_default())
    }

    async fn duplicates(&self, id: &LibraryId) -> Result<Vec<DuplicateCandidate>, LibraryError> {
        let state = self.state.lock().unwrap();
        require_library(&state, id)?;
        Ok(state.duplicates.get(id).cloned().unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{MovieId, TitleId};
    use domain::library::{DuplicateCandidateId, LibraryKind, UnmatchedFileId, WatcherStrategy};

    fn library(id: &str) -> Library {
        Library {
            id: LibraryId(id.into()),
            name: format!("Lib {id}"),
            kind: LibraryKind::Movie,
            roots: vec!["/media".into()],
            watcher: WatcherStrategy::Manual,
            scan_schedule: None,
            metadata_sources: Vec::new(),
        }
    }

    #[tokio::test]
    async fn list_and_detail() {
        let svc = MockLibraryService::new();
        svc.add_library(library("l1"));
        assert_eq!(svc.libraries().await.unwrap().len(), 1);
        assert!(svc.library(&LibraryId("l1".into())).await.is_ok());
        assert!(matches!(
            svc.library(&LibraryId("x".into())).await.unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn scan_state_defaults_to_idle() {
        let svc = MockLibraryService::new();
        svc.add_library(library("l1"));
        let state = svc.scan_state(&LibraryId("l1".into())).await.unwrap();
        assert_eq!(state.status, ScanStatus::Idle);
        assert!(matches!(
            svc.scan_state(&LibraryId("x".into())).await.unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn trigger_scan_sets_running_then_conflicts() {
        let svc = MockLibraryService::new();
        svc.add_library(library("l1"));
        svc.trigger_scan(&LibraryId("l1".into())).await.unwrap();
        let state = svc.scan_state(&LibraryId("l1".into())).await.unwrap();
        assert_eq!(state.status, ScanStatus::Running);

        assert!(matches!(
            svc.trigger_scan(&LibraryId("l1".into())).await.unwrap_err(),
            LibraryError::ScanInProgress
        ));
        assert!(matches!(
            svc.trigger_scan(&LibraryId("x".into())).await.unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn unmatched_and_duplicates() {
        let svc = MockLibraryService::new();
        let id = LibraryId("l1".into());
        svc.add_library(library("l1"));
        assert!(svc.unmatched(&id).await.unwrap().is_empty());
        assert!(svc.duplicates(&id).await.unwrap().is_empty());

        svc.add_unmatched(
            &id,
            UnmatchedFile {
                id: UnmatchedFileId("uf1".into()),
                library: id.clone(),
                path: "/media/x.mkv".into(),
                candidates: Vec::new(),
            },
        );
        svc.add_duplicate(
            &id,
            DuplicateCandidate {
                id: DuplicateCandidateId("d1".into()),
                title: TitleId::Movie(MovieId("m1".into())),
                paths: vec!["/a.mkv".into(), "/b.mkv".into()],
            },
        );
        assert_eq!(svc.unmatched(&id).await.unwrap().len(), 1);
        assert_eq!(svc.duplicates(&id).await.unwrap().len(), 1);

        assert!(matches!(
            svc.unmatched(&LibraryId("x".into())).await.unwrap_err(),
            LibraryError::NotFound
        ));
        assert!(matches!(
            svc.duplicates(&LibraryId("x".into())).await.unwrap_err(),
            LibraryError::NotFound
        ));
    }
}

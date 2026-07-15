use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use domain::catalog::{TitleId, TitleRef};
use domain::common::{Page, PageRequest};
use domain::error::LibraryError;
use domain::job::Job;
use domain::library::{
    DuplicateCandidate, DuplicateCandidateId, Library, LibraryId, LibraryUpdate, NewLibrary,
    ResolveCandidate, ResolveTarget, ScanState, ScanStatus, UnmatchedFile, UnmatchedFileId,
};
use domain::metadata::{ExternalId, MediaKind};
use domain::service::LibraryService;
use domain::user::Principal;

use crate::page::paginate;

#[derive(Debug, Default)]
struct State {
    libraries: Vec<Library>,
    library_seq: u32,
    scans: HashMap<LibraryId, ScanState>,
    unmatched: HashMap<LibraryId, Vec<UnmatchedFile>>,
    duplicates: HashMap<LibraryId, Vec<DuplicateCandidate>>,
    jobs: Vec<Job>,
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

    pub fn add_job(&self, job: Job) {
        self.state.lock().unwrap().jobs.push(job);
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
    async fn libraries(&self, _caller: &Principal) -> Result<Vec<Library>, LibraryError> {
        Ok(self.state.lock().unwrap().libraries.clone())
    }

    async fn library(&self, _caller: &Principal, id: &LibraryId) -> Result<Library, LibraryError> {
        self.state
            .lock()
            .unwrap()
            .libraries
            .iter()
            .find(|l| &l.id == id)
            .cloned()
            .ok_or(LibraryError::NotFound)
    }

    async fn create_library(
        &self,
        _caller: &Principal,
        input: NewLibrary,
    ) -> Result<Library, LibraryError> {
        let mut state = self.state.lock().unwrap();
        state.library_seq += 1;
        let library = Library {
            id: LibraryId(format!("lib-{}", state.library_seq)),
            name: input.name,
            kind: input.kind,
            roots: input.roots,
            watcher: input.watcher,
            scan_schedule: input.scan_schedule,
            metadata_sources: input.metadata_sources,
        };
        state.libraries.push(library.clone());
        Ok(library)
    }

    async fn update_library(
        &self,
        _caller: &Principal,
        id: &LibraryId,
        update: LibraryUpdate,
    ) -> Result<Library, LibraryError> {
        let mut state = self.state.lock().unwrap();
        let library = state
            .libraries
            .iter_mut()
            .find(|l| &l.id == id)
            .ok_or(LibraryError::NotFound)?;
        library.name = update.name;
        library.kind = update.kind;
        library.roots = update.roots;
        library.watcher = update.watcher;
        library.scan_schedule = update.scan_schedule;
        library.metadata_sources = update.metadata_sources;
        Ok(library.clone())
    }

    async fn delete_library(
        &self,
        _caller: &Principal,
        id: &LibraryId,
    ) -> Result<(), LibraryError> {
        let mut state = self.state.lock().unwrap();
        let before = state.libraries.len();
        state.libraries.retain(|l| &l.id != id);
        if state.libraries.len() == before {
            return Err(LibraryError::NotFound);
        }
        Ok(())
    }

    async fn scan_state(
        &self,
        _caller: &Principal,
        id: &LibraryId,
    ) -> Result<ScanState, LibraryError> {
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

    async fn trigger_scan(&self, _caller: &Principal, id: &LibraryId) -> Result<(), LibraryError> {
        let mut state = self.state.lock().unwrap();
        require_library(&state, id)?;
        if matches!(state.scans.get(id), Some(s) if matches!(s.status, ScanStatus::Queued | ScanStatus::Running))
        {
            return Err(LibraryError::ScanInProgress);
        }
        state.scans.insert(
            id.clone(),
            ScanState {
                library: id.clone(),
                status: ScanStatus::Queued,
                progress: 0.0,
                last_scanned_at: None,
                error: None,
            },
        );
        Ok(())
    }

    async fn unmatched(
        &self,
        _caller: &Principal,
        id: &LibraryId,
        page: PageRequest,
    ) -> Result<Page<UnmatchedFile>, LibraryError> {
        let state = self.state.lock().unwrap();
        require_library(&state, id)?;
        let all = state.unmatched.get(id).cloned().unwrap_or_default();
        Ok(paginate(&all, page))
    }

    async fn duplicates(
        &self,
        _caller: &Principal,
        id: &LibraryId,
        page: PageRequest,
    ) -> Result<Page<DuplicateCandidate>, LibraryError> {
        let state = self.state.lock().unwrap();
        require_library(&state, id)?;
        let all = state.duplicates.get(id).cloned().unwrap_or_default();
        Ok(paginate(&all, page))
    }

    async fn unmatched_candidates(
        &self,
        _caller: &Principal,
        id: &LibraryId,
        unmatched: &UnmatchedFileId,
        _query: Option<String>,
    ) -> Result<Vec<ResolveCandidate>, LibraryError> {
        let state = self.state.lock().unwrap();
        require_library(&state, id)?;
        let file = state
            .unmatched
            .get(id)
            .and_then(|files| files.iter().find(|file| &file.id == unmatched))
            .ok_or(LibraryError::NotFound)?;
        Ok(file
            .candidates
            .iter()
            .map(|candidate| ResolveCandidate {
                target: ResolveTarget::Existing(candidate.title.clone()),
                title: candidate.label.clone(),
                year: None,
                kind: match &candidate.title {
                    TitleId::Movie(_) => MediaKind::Movie,
                    TitleId::Episode(_) => MediaKind::Series,
                },
            })
            .collect())
    }

    async fn resolve_unmatched(
        &self,
        _caller: &Principal,
        id: &LibraryId,
        unmatched: &UnmatchedFileId,
        _target: ResolveTarget,
    ) -> Result<(), LibraryError> {
        let state = self.state.lock().unwrap();
        require_library(&state, id)?;
        let exists = state
            .unmatched
            .get(id)
            .is_some_and(|files| files.iter().any(|file| &file.id == unmatched));
        if exists {
            Ok(())
        } else {
            Err(LibraryError::NotFound)
        }
    }

    async fn dismiss_duplicate(
        &self,
        _caller: &Principal,
        id: &LibraryId,
        duplicate: &DuplicateCandidateId,
    ) -> Result<(), LibraryError> {
        let mut state = self.state.lock().unwrap();
        require_library(&state, id)?;
        if let Some(duplicates) = state.duplicates.get_mut(id) {
            duplicates.retain(|candidate| &candidate.id != duplicate);
        }
        Ok(())
    }

    async fn resolve_duplicate(
        &self,
        _caller: &Principal,
        id: &LibraryId,
        duplicate: &DuplicateCandidateId,
    ) -> Result<(), LibraryError> {
        let mut state = self.state.lock().unwrap();
        require_library(&state, id)?;
        if let Some(duplicates) = state.duplicates.get_mut(id) {
            duplicates.retain(|candidate| &candidate.id != duplicate);
        }
        Ok(())
    }

    async fn reidentify(
        &self,
        _caller: &Principal,
        _title: TitleRef,
        _external_id: Option<ExternalId>,
    ) -> Result<(), LibraryError> {
        Ok(())
    }

    async fn jobs(&self, _caller: &Principal) -> Result<Vec<Job>, LibraryError> {
        Ok(self.state.lock().unwrap().jobs.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{MovieId, TitleId};
    use domain::library::{DuplicateCandidateId, LibraryKind, UnmatchedFileId, WatcherStrategy};
    use domain::user::{Role, UserId};

    fn principal() -> Principal {
        Principal {
            user: UserId("u1".into()),
            role: Role::Admin,
        }
    }

    fn page() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 10,
        }
    }

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
        assert_eq!(svc.libraries(&principal()).await.unwrap().len(), 1);
        assert!(
            svc.library(&principal(), &LibraryId("l1".into()))
                .await
                .is_ok()
        );
        assert!(matches!(
            svc.library(&principal(), &LibraryId("x".into()))
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn create_update_delete() {
        let svc = MockLibraryService::new();
        let created = svc
            .create_library(
                &principal(),
                NewLibrary {
                    name: "New".into(),
                    kind: LibraryKind::Movie,
                    roots: vec!["/n".into()],
                    watcher: WatcherStrategy::Manual,
                    scan_schedule: None,
                    metadata_sources: Vec::new(),
                },
            )
            .await
            .unwrap();
        assert_eq!(created.id, LibraryId("lib-1".into()));

        let updated = svc
            .update_library(
                &principal(),
                &created.id,
                LibraryUpdate {
                    name: "Renamed".into(),
                    kind: LibraryKind::Tv,
                    roots: Vec::new(),
                    watcher: WatcherStrategy::Manual,
                    scan_schedule: None,
                    metadata_sources: Vec::new(),
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.name, "Renamed");
        assert_eq!(updated.kind, LibraryKind::Tv);
        assert!(matches!(
            svc.update_library(
                &principal(),
                &LibraryId("x".into()),
                LibraryUpdate {
                    name: "z".into(),
                    kind: LibraryKind::Movie,
                    roots: Vec::new(),
                    watcher: WatcherStrategy::Manual,
                    scan_schedule: None,
                    metadata_sources: Vec::new(),
                },
            )
            .await
            .unwrap_err(),
            LibraryError::NotFound
        ));

        svc.delete_library(&principal(), &created.id).await.unwrap();
        assert!(matches!(
            svc.delete_library(&principal(), &created.id)
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn jobs_returns_seeded() {
        use domain::job::{Job, JobId, JobKind, JobPriority, JobStatus};
        use jiff::Timestamp;

        let svc = MockLibraryService::new();
        assert!(svc.jobs(&principal()).await.unwrap().is_empty());
        let now = Timestamp::UNIX_EPOCH;
        svc.add_job(Job {
            id: JobId("j1".into()),
            kind: JobKind::LibraryScan,
            status: JobStatus::Succeeded,
            priority: JobPriority::Normal,
            payload: "lib1".into(),
            attempts: 1,
            progress: 1.0,
            available_at: now,
            last_error: None,
            created_at: now,
            updated_at: now,
        });
        assert_eq!(svc.jobs(&principal()).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn scan_state_defaults_to_idle() {
        let svc = MockLibraryService::new();
        svc.add_library(library("l1"));
        let state = svc
            .scan_state(&principal(), &LibraryId("l1".into()))
            .await
            .unwrap();
        assert_eq!(state.status, ScanStatus::Idle);
        assert!(matches!(
            svc.scan_state(&principal(), &LibraryId("x".into()))
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn trigger_scan_sets_queued_then_conflicts() {
        let svc = MockLibraryService::new();
        svc.add_library(library("l1"));
        svc.trigger_scan(&principal(), &LibraryId("l1".into()))
            .await
            .unwrap();
        let state = svc
            .scan_state(&principal(), &LibraryId("l1".into()))
            .await
            .unwrap();
        assert_eq!(state.status, ScanStatus::Queued);

        assert!(matches!(
            svc.trigger_scan(&principal(), &LibraryId("l1".into()))
                .await
                .unwrap_err(),
            LibraryError::ScanInProgress
        ));
        assert!(matches!(
            svc.trigger_scan(&principal(), &LibraryId("x".into()))
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn unmatched_and_duplicates() {
        let svc = MockLibraryService::new();
        let id = LibraryId("l1".into());
        svc.add_library(library("l1"));
        assert!(
            svc.unmatched(&principal(), &id, page())
                .await
                .unwrap()
                .items
                .is_empty()
        );
        assert!(
            svc.duplicates(&principal(), &id, page())
                .await
                .unwrap()
                .items
                .is_empty()
        );

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
        assert_eq!(
            svc.unmatched(&principal(), &id, page())
                .await
                .unwrap()
                .total,
            1
        );
        assert_eq!(
            svc.duplicates(&principal(), &id, page())
                .await
                .unwrap()
                .total,
            1
        );

        assert!(matches!(
            svc.unmatched(&principal(), &LibraryId("x".into()), page())
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
        assert!(matches!(
            svc.duplicates(&principal(), &LibraryId("x".into()), page())
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn resolve_unmatched_requires_library_and_file() {
        use domain::catalog::{MovieId, TitleId};
        use domain::library::{ResolveTarget, UnmatchedFile, UnmatchedFileId};

        let svc = MockLibraryService::new();
        let id = LibraryId("l1".into());
        svc.add_library(library("l1"));
        let uid = UnmatchedFileId("uf1".into());
        let target = || ResolveTarget::Existing(TitleId::Movie(MovieId("m1".into())));

        assert!(matches!(
            svc.resolve_unmatched(&principal(), &id, &uid, target())
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));

        svc.add_unmatched(
            &id,
            UnmatchedFile {
                id: uid.clone(),
                library: id.clone(),
                path: "/m/x.mkv".into(),
                candidates: Vec::new(),
            },
        );
        svc.resolve_unmatched(&principal(), &id, &uid, target())
            .await
            .unwrap();

        assert!(matches!(
            svc.resolve_unmatched(&principal(), &LibraryId("x".into()), &uid, target())
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn reidentify_is_accepted() {
        use domain::catalog::MovieId;

        let svc = MockLibraryService::new();
        svc.reidentify(&principal(), TitleRef::Movie(MovieId("m1".into())), None)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn dismiss_and_resolve_duplicate_are_library_gated() {
        use domain::catalog::{MovieId, TitleId};

        let svc = MockLibraryService::new();
        let id = LibraryId("l1".into());
        svc.add_library(library("l1"));
        svc.add_duplicate(
            &id,
            DuplicateCandidate {
                id: DuplicateCandidateId("d1".into()),
                title: TitleId::Movie(MovieId("m1".into())),
                paths: vec!["/a.mkv".into()],
            },
        );

        svc.dismiss_duplicate(&principal(), &id, &DuplicateCandidateId("d1".into()))
            .await
            .unwrap();
        assert!(
            svc.duplicates(&principal(), &id, page())
                .await
                .unwrap()
                .items
                .is_empty()
        );
        svc.resolve_duplicate(&principal(), &id, &DuplicateCandidateId("gone".into()))
            .await
            .unwrap();

        let missing = LibraryId("x".into());
        assert!(matches!(
            svc.dismiss_duplicate(&principal(), &missing, &DuplicateCandidateId("d1".into()))
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
        assert!(matches!(
            svc.resolve_duplicate(&principal(), &missing, &DuplicateCandidateId("d1".into()))
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }
}

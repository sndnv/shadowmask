use domain::common::{Page, PageRequest};
use domain::error::LibraryError;
use domain::job::{Job, JobId, JobKind, JobPriority, JobStatus};
use domain::library::{
    DuplicateCandidate, Library, LibraryId, ScanState, ScanStatus, UnmatchedFile,
};
use domain::repository::{JobRepository, LibraryRepository, UserRepository};
use domain::service::LibraryService;
use domain::user::Principal;
use jiff::Timestamp;
use uuid::Uuid;

use crate::acl;

#[derive(Clone)]
pub struct LibraryServiceImpl<L, U, J> {
    libraries: L,
    users: U,
    jobs: J,
}

impl<L, U, J> LibraryServiceImpl<L, U, J> {
    pub fn new(libraries: L, users: U, jobs: J) -> Self {
        Self {
            libraries,
            users,
            jobs,
        }
    }
}

impl<L, U, J> LibraryServiceImpl<L, U, J>
where
    L: LibraryRepository + Sync,
    U: UserRepository + Sync,
    J: JobRepository + Sync,
{
    async fn visible_to(&self, caller: &Principal, id: &LibraryId) -> Result<bool, LibraryError> {
        if acl::is_admin(caller) {
            return Ok(true);
        }
        let access: Vec<LibraryId> = self
            .users
            .list_library_access(&caller.user)
            .await?
            .into_iter()
            .map(|entry| entry.library)
            .collect();
        Ok(acl::can_access_library(&access, id))
    }

    async fn require_library(
        &self,
        caller: &Principal,
        id: &LibraryId,
    ) -> Result<Library, LibraryError> {
        let library = self
            .libraries
            .get(id)
            .await?
            .ok_or(LibraryError::NotFound)?;
        if self.visible_to(caller, id).await? {
            Ok(library)
        } else {
            Err(LibraryError::NotFound)
        }
    }
}

impl<L, U, J> LibraryService for LibraryServiceImpl<L, U, J>
where
    L: LibraryRepository + Sync,
    U: UserRepository + Sync,
    J: JobRepository + Sync,
{
    async fn libraries(&self, caller: &Principal) -> Result<Vec<Library>, LibraryError> {
        let all = self.libraries.list().await?;
        if acl::is_admin(caller) {
            return Ok(all);
        }
        let access: Vec<LibraryId> = self
            .users
            .list_library_access(&caller.user)
            .await?
            .into_iter()
            .map(|entry| entry.library)
            .collect();
        Ok(all
            .into_iter()
            .filter(|library| acl::can_access_library(&access, &library.id))
            .collect())
    }

    async fn library(&self, caller: &Principal, id: &LibraryId) -> Result<Library, LibraryError> {
        self.require_library(caller, id).await
    }

    async fn scan_state(
        &self,
        caller: &Principal,
        id: &LibraryId,
    ) -> Result<ScanState, LibraryError> {
        self.require_library(caller, id).await?;
        Ok(self.libraries.scan_state(id).await?.unwrap_or(ScanState {
            library: id.clone(),
            status: ScanStatus::Idle,
            progress: 0.0,
            last_scanned_at: None,
            error: None,
        }))
    }

    async fn trigger_scan(&self, caller: &Principal, id: &LibraryId) -> Result<(), LibraryError> {
        self.require_library(caller, id).await?;
        if matches!(
            self.libraries.scan_state(id).await?,
            Some(state) if state.status == ScanStatus::Running
        ) {
            return Err(LibraryError::ScanInProgress);
        }
        let now = Timestamp::now();
        self.libraries
            .save_scan_state(ScanState {
                library: id.clone(),
                status: ScanStatus::Running,
                progress: 0.0,
                last_scanned_at: None,
                error: None,
            })
            .await?;
        self.jobs
            .enqueue(Job {
                id: JobId(Uuid::new_v4().to_string()),
                kind: JobKind::LibraryScan,
                status: JobStatus::Queued,
                priority: JobPriority::Normal,
                payload: id.0.clone(),
                attempts: 0,
                progress: 0.0,
                available_at: now,
                last_error: None,
                created_at: now,
                updated_at: now,
            })
            .await?;
        Ok(())
    }

    async fn unmatched(
        &self,
        caller: &Principal,
        id: &LibraryId,
        page: PageRequest,
    ) -> Result<Page<UnmatchedFile>, LibraryError> {
        self.require_library(caller, id).await?;
        Ok(self.libraries.list_unmatched(id, page).await?)
    }

    async fn duplicates(
        &self,
        caller: &Principal,
        id: &LibraryId,
        page: PageRequest,
    ) -> Result<Page<DuplicateCandidate>, LibraryError> {
        self.require_library(caller, id).await?;
        Ok(self.libraries.list_duplicates(id, page).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{MockJobStore, MockLibraryRepo, MockUserRepo};
    use domain::library::{LibraryKind, WatcherStrategy};
    use domain::repository::JobRepository;
    use domain::user::{Role, User, UserId};
    use jiff::Timestamp;

    type Svc = LibraryServiceImpl<MockLibraryRepo, MockUserRepo, MockJobStore>;

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

    fn user(id: &str) -> User {
        User {
            id: UserId(id.into()),
            username: id.into(),
            password_hash: "hash".into(),
            role: Role::User,
            max_content_rating: None,
            preferred_audio: Vec::new(),
            preferred_subtitle: Vec::new(),
            concurrent_stream_limit: None,
            bitrate_cap: None,
            created_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn admin() -> Principal {
        Principal {
            user: UserId("admin".into()),
            role: Role::Admin,
        }
    }

    fn member() -> Principal {
        Principal {
            user: UserId("u1".into()),
            role: Role::User,
        }
    }

    fn page() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 10,
        }
    }

    async fn seeded() -> Svc {
        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library("lib1"));
        libraries.insert_library(library("lib2"));
        let users = MockUserRepo::new();
        users.insert(user("u1"));
        users
            .set_library_access(&UserId("u1".into()), &[LibraryId("lib1".into())])
            .await
            .unwrap();
        LibraryServiceImpl::new(libraries, users, MockJobStore::new())
    }

    #[tokio::test]
    async fn admin_sees_all_member_filtered() {
        let svc = seeded().await;
        assert_eq!(svc.libraries(&admin()).await.unwrap().len(), 2);
        let visible = svc.libraries(&member()).await.unwrap();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id, LibraryId("lib1".into()));
    }

    #[tokio::test]
    async fn detail_hides_disallowed_as_not_found() {
        let svc = seeded().await;
        assert!(
            svc.library(&member(), &LibraryId("lib1".into()))
                .await
                .is_ok()
        );
        assert!(matches!(
            svc.library(&member(), &LibraryId("lib2".into()))
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
        assert!(matches!(
            svc.library(&admin(), &LibraryId("missing".into()))
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn scan_state_defaults_then_trigger_enqueues_and_conflicts() {
        let svc = seeded().await;
        let id = LibraryId("lib1".into());
        let state = svc.scan_state(&admin(), &id).await.unwrap();
        assert_eq!(state.status, ScanStatus::Idle);

        svc.trigger_scan(&admin(), &id).await.unwrap();
        assert_eq!(svc.jobs.list().await.unwrap().len(), 1);
        assert_eq!(svc.jobs.list().await.unwrap()[0].kind, JobKind::LibraryScan);
        assert_eq!(
            svc.scan_state(&admin(), &id).await.unwrap().status,
            ScanStatus::Running
        );
        assert!(matches!(
            svc.trigger_scan(&admin(), &id).await.unwrap_err(),
            LibraryError::ScanInProgress
        ));
    }

    #[tokio::test]
    async fn unmatched_and_duplicates_gated() {
        let svc = seeded().await;
        let id = LibraryId("lib1".into());
        assert!(
            svc.unmatched(&admin(), &id, page())
                .await
                .unwrap()
                .items
                .is_empty()
        );
        assert!(
            svc.duplicates(&admin(), &id, page())
                .await
                .unwrap()
                .items
                .is_empty()
        );
        assert!(matches!(
            svc.unmatched(&member(), &LibraryId("lib2".into()), page())
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
        assert!(matches!(
            svc.duplicates(&member(), &LibraryId("lib2".into()), page())
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn backend_errors_propagate() {
        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library("lib1"));
        libraries.set_fail_get();
        let svc = LibraryServiceImpl::new(libraries, MockUserRepo::new(), MockJobStore::new());
        assert!(matches!(
            svc.library(&admin(), &LibraryId("lib1".into()))
                .await
                .unwrap_err(),
            LibraryError::Repository(_)
        ));
    }
}

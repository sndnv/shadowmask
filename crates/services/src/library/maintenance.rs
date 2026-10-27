use std::collections::HashSet;
use std::sync::Arc;

use domain::catalog::{CatalogClean, CleanScope, LibraryPurge, TitleSet, VersionId};
use domain::common::PageRequest;
use domain::error::{MaintenanceError, RepositoryError};
use domain::job::{Job, JobCanceller, JobKind};
use domain::library::{
    CancelScope, CancelledJobs, CleanReport, LibraryId, PurgeReport, ScanState, ScanStatus,
};
use domain::repository::{
    CatalogRepository, JobRepository, LibraryRepository, UserDataStore, UserRepository,
};
use domain::service::MaintenanceService;
use domain::user::Principal;
use jiff::Timestamp;

use super::job_scope::JobScope;
use super::scan_queue::release_scan;
use crate::acl;

const PURGE_BLOCKERS: [JobKind; 12] = [
    JobKind::LibraryScan,
    JobKind::Ingest,
    JobKind::Relink,
    JobKind::Fetch,
    JobKind::Metadata,
    JobKind::Artwork,
    JobKind::Upscale,
    JobKind::Subtitles,
    JobKind::Translation,
    JobKind::Combine,
    JobKind::Dedup,
    JobKind::SearchReindex,
];

const CLEAN_BLOCKERS: [JobKind; 8] = [
    JobKind::LibraryScan,
    JobKind::Ingest,
    JobKind::Relink,
    JobKind::Fetch,
    JobKind::Metadata,
    JobKind::Artwork,
    JobKind::Dedup,
    JobKind::SearchReindex,
];

fn holding(id: &LibraryId, held: Option<&ScanState>, status: ScanStatus) -> ScanState {
    ScanState {
        library: id.clone(),
        status,
        progress: 0.0,
        started_at: Some(Timestamp::now()),
        last_scanned_at: held.and_then(|state| state.last_scanned_at),
        error: None,
    }
}

fn held_by(status: ScanStatus) -> Option<MaintenanceError> {
    match status {
        ScanStatus::Purging => Some(MaintenanceError::Purging),
        ScanStatus::Cleaning => Some(MaintenanceError::Cleaning),
        _ => None,
    }
}

fn scanning(blockers: Vec<Job>) -> MaintenanceError {
    if blockers.is_empty() { MaintenanceError::Scanning } else { MaintenanceError::Busy(blockers) }
}

fn idle(id: &LibraryId) -> ScanState {
    ScanState {
        library: id.clone(),
        status: ScanStatus::Idle,
        progress: 0.0,
        started_at: None,
        last_scanned_at: None,
        error: None,
    }
}

fn report(purged: &LibraryPurge, wiped_users: u64) -> PurgeReport {
    PurgeReport {
        movies: purged.titles.movies.len() as u64,
        series: purged.titles.series.len() as u64,
        seasons: purged.titles.seasons.len() as u64,
        episodes: purged.titles.episodes.len() as u64,
        subtitles: purged.subtitles,
        wiped_users,
    }
}

fn clean_report(cleaned: &CatalogClean, wiped_users: u64) -> CleanReport {
    CleanReport {
        versions: cleaned.versions.len() as u64,
        movies: cleaned.titles.movies.len() as u64,
        series: cleaned.titles.series.len() as u64,
        seasons: cleaned.titles.seasons.len() as u64,
        episodes: cleaned.titles.episodes.len() as u64,
        wiped_users,
    }
}

#[derive(Clone)]
pub struct MaintenanceServiceImpl<L, U, J, C, D> {
    libraries: L,
    users: U,
    jobs: J,
    catalog: C,
    user_data: D,
    canceller: Arc<dyn JobCanceller>,
}

impl<L, U, J, C, D> MaintenanceServiceImpl<L, U, J, C, D> {
    pub fn new(
        libraries: L,
        users: U,
        jobs: J,
        catalog: C,
        user_data: D,
        canceller: Arc<dyn JobCanceller>,
    ) -> Self {
        Self { libraries, users, jobs, catalog, user_data, canceller }
    }
}

impl<L, U, J, C, D> MaintenanceServiceImpl<L, U, J, C, D>
where
    L: LibraryRepository + Sync,
    U: UserRepository + Sync,
    J: JobRepository + Sync,
    C: CatalogRepository + Sync,
    D: UserDataStore + Sync,
{
    async fn require_library(
        &self,
        caller: &Principal,
        id: &LibraryId,
    ) -> Result<(), MaintenanceError> {
        if !acl::is_admin(caller) {
            return Err(MaintenanceError::Forbidden);
        }
        self.libraries.get(id).await?.ok_or(MaintenanceError::NotFound)?;
        Ok(())
    }

    async fn blockers(
        &self,
        id: &LibraryId,
        kinds: &[JobKind],
    ) -> Result<Vec<Job>, RepositoryError> {
        let active = self.jobs.list_active(kinds).await?;
        if active.is_empty() {
            return Ok(active);
        }
        let footprint = self.catalog.library_footprint(id).await?;
        Ok(active
            .into_iter()
            .filter(|job| JobScope::of(job).blocks_purge(id, &footprint))
            .collect())
    }

    async fn clean_blockers(&self, scope: &CleanScope) -> Result<Vec<Job>, RepositoryError> {
        let CleanScope::Library(id) = scope else {
            let active = self.jobs.list_active(&CLEAN_BLOCKERS).await?;
            return Ok(active
                .into_iter()
                .filter(|job| JobScope::of(job).blocks_full_clean())
                .collect());
        };
        self.blockers(id, &CLEAN_BLOCKERS).await
    }

    async fn purge_held(
        &self,
        id: &LibraryId,
        wipe_user_data: bool,
    ) -> Result<PurgeReport, MaintenanceError> {
        let blockers = self.blockers(id, &PURGE_BLOCKERS).await?;
        if !blockers.is_empty() {
            return Err(MaintenanceError::Busy(blockers));
        }
        let purged = self.catalog.purge_library(id).await?;
        let wiped_users =
            if wipe_user_data { self.wipe(&purged.titles, &purged.versions).await? } else { 0 };
        self.libraries.clear_scan_data(id).await?;
        Ok(report(&purged, wiped_users))
    }

    async fn clean_held(
        &self,
        scope: &CleanScope,
        wipe_user_data: bool,
    ) -> Result<CleanReport, MaintenanceError> {
        let blockers = self.clean_blockers(scope).await?;
        if !blockers.is_empty() {
            return Err(MaintenanceError::Busy(blockers));
        }
        let cleaned = self.catalog.clean_catalog(scope).await?;
        let wiped_users =
            if wipe_user_data { self.wipe(&cleaned.titles, &cleaned.versions).await? } else { 0 };
        Ok(clean_report(&cleaned, wiped_users))
    }

    async fn clean_libraries(
        &self,
        ids: &[LibraryId],
        scope: &CleanScope,
        wipe_user_data: bool,
    ) -> Result<CleanReport, MaintenanceError> {
        let mut states = Vec::with_capacity(ids.len());
        for id in ids {
            let state = self.libraries.scan_state(id).await?;
            if let Some(status) = state.as_ref().map(|state| state.status)
                && status.is_busy()
            {
                return Err(match held_by(status) {
                    Some(held) => held,
                    None => scanning(self.clean_blockers(scope).await?),
                });
            }
            states.push((id, state));
        }
        let mut held = Vec::with_capacity(states.len());
        let mut cleaned = Ok(CleanReport::default());
        for (id, state) in states {
            let hold = holding(id, state.as_ref(), ScanStatus::Cleaning);
            if let Err(err) = self.libraries.save_scan_state(hold).await {
                cleaned = Err(MaintenanceError::Repository(err));
                break;
            }
            held.push((id, state));
        }
        if cleaned.is_ok() {
            cleaned = self.clean_held(scope, wipe_user_data).await;
        }
        for (id, state) in held {
            self.release(id, state, "clean").await;
        }
        cleaned
    }

    async fn wipe(
        &self,
        titles: &TitleSet,
        versions: &HashSet<VersionId>,
    ) -> Result<u64, RepositoryError> {
        if versions.is_empty() && *titles == TitleSet::default() {
            return Ok(0);
        }
        let users = self.users.list(PageRequest::ALL).await?.items;
        for user in &users {
            self.user_data.forget(&user.id, titles, versions).await?;
        }
        Ok(users.len() as u64)
    }

    async fn release(&self, id: &LibraryId, held: Option<ScanState>, after: &str) {
        let restored = held.unwrap_or_else(|| idle(id));
        if let Err(err) = self.libraries.save_scan_state(restored).await {
            #[rustfmt::skip]
            tracing::warn!("releasing library [{}] after a {after} failed: [{err}]", id.0);
        }
    }
}

impl<L, U, J, C, D> MaintenanceService for MaintenanceServiceImpl<L, U, J, C, D>
where
    L: LibraryRepository + Sync,
    U: UserRepository + Sync,
    J: JobRepository + Sync,
    C: CatalogRepository + Sync,
    D: UserDataStore + Sync,
{
    async fn purge_library(
        &self,
        caller: &Principal,
        id: &LibraryId,
        wipe_user_data: bool,
    ) -> Result<PurgeReport, MaintenanceError> {
        self.require_library(caller, id).await?;
        let held = self.libraries.scan_state(id).await?;
        if let Some(status) = held.as_ref().map(|state| state.status)
            && status.is_busy()
        {
            return Err(match held_by(status) {
                Some(held) => held,
                None => scanning(self.blockers(id, &PURGE_BLOCKERS).await?),
            });
        }
        self.libraries.save_scan_state(holding(id, held.as_ref(), ScanStatus::Purging)).await?;
        let purged = self.purge_held(id, wipe_user_data).await;
        let Ok(done) = &purged else {
            self.release(id, held, "purge that did not finish").await;
            return purged;
        };
        #[rustfmt::skip]
        tracing::info!("library [{}] purged: [{}] movies, [{}] series, [{}] seasons, [{}] episodes, [{}] subtitles; user data of [{}] users wiped", id.0, done.movies, done.series, done.seasons, done.episodes, done.subtitles, done.wiped_users);
        purged
    }

    async fn clean_library(
        &self,
        caller: &Principal,
        id: &LibraryId,
        wipe_user_data: bool,
    ) -> Result<CleanReport, MaintenanceError> {
        self.require_library(caller, id).await?;
        let scope = CleanScope::Library(id.clone());
        let done = self.clean_libraries(std::slice::from_ref(id), &scope, wipe_user_data).await?;
        #[rustfmt::skip]
        tracing::info!("library [{}] cleaned: [{}] versions, [{}] movies, [{}] series, [{}] seasons, [{}] episodes; user data of [{}] users wiped", id.0, done.versions, done.movies, done.series, done.seasons, done.episodes, done.wiped_users);
        Ok(done)
    }

    async fn clean_all(
        &self,
        caller: &Principal,
        wipe_user_data: bool,
    ) -> Result<CleanReport, MaintenanceError> {
        if !acl::is_admin(caller) {
            return Err(MaintenanceError::Forbidden);
        }
        let ids: Vec<LibraryId> =
            self.libraries.list().await?.into_iter().map(|library| library.id).collect();
        let done = self.clean_libraries(&ids, &CleanScope::All, wipe_user_data).await?;
        #[rustfmt::skip]
        tracing::info!("every library cleaned: [{}] versions, [{}] movies, [{}] series, [{}] seasons, [{}] episodes; user data of [{}] users wiped", done.versions, done.movies, done.series, done.seasons, done.episodes, done.wiped_users);
        Ok(done)
    }

    async fn cancel_library_jobs(
        &self,
        caller: &Principal,
        id: &LibraryId,
        scope: CancelScope,
    ) -> Result<CancelledJobs, MaintenanceError> {
        self.require_library(caller, id).await?;
        let kinds: &[JobKind] = match scope {
            CancelScope::All => &JobKind::ALL,
            CancelScope::PurgeBlockers => &PURGE_BLOCKERS,
            CancelScope::CleanBlockers => &CLEAN_BLOCKERS,
        };
        let active = self.jobs.list_active(kinds).await?;
        let footprint = self.catalog.library_footprint(id).await?;
        let now = Timestamp::now();
        let mut outcome = CancelledJobs::default();
        let mut scan_cancelled = false;
        for job in active.iter().filter(|job| JobScope::of(job).is_of(id, &footprint)) {
            if self.jobs.cancel(&job.id, now).await? {
                outcome.cancelled += 1;
                scan_cancelled |= job.kind == JobKind::LibraryScan;
            } else if job.kind.is_process_killable() && self.canceller.request_cancel(&job.id) {
                outcome.signalled += 1;
            } else {
                outcome.running += 1;
            }
        }
        if scan_cancelled {
            release_scan(&self.libraries, id).await?;
        }
        #[rustfmt::skip]
        tracing::info!("jobs of library [{}] cancelled: [{}] queued, [{}] signalled, [{}] left running", id.0, outcome.cancelled, outcome.signalled, outcome.running);
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::Mutex;

    use domain::catalog::{Movie, MovieId, TitleId, Version, VersionId};
    use domain::common::Quality;
    use domain::job::{JobId, JobPriority, JobStatus};
    use domain::library::{
        DiscoveredFile, Library, LibraryKind, LibraryOrigin, ScanMode, ScanReport, UnmatchedFile,
        UnmatchedFileId, WatcherStrategy,
    };
    use domain::media::{ProbeResult, TrickplayAsset};
    use domain::user::{Role, User, UserId};
    use mocks::{
        MockCatalogRepo, MockJobStore, MockLibraryRepo, MockMetadataProvider, MockUserDataStore,
        MockUserRepo,
    };

    use super::*;
    use crate::job::queued_job;
    use crate::library::{
        ArtworkJobPayload, Enricher, IngestJobPayload, MetadataJobPayload, RelinkJobPayload,
        ScanEnricher, ScanJobPayload, SubtitleJobPayload, TrickplayJobPayload, derive_id,
    };

    type Svc = MaintenanceServiceImpl<
        MockLibraryRepo,
        MockUserRepo,
        MockJobStore,
        MockCatalogRepo,
        MockUserDataStore,
    >;

    #[derive(Clone)]
    struct RecordingCanceller {
        accepts: bool,
        calls: Arc<Mutex<Vec<JobId>>>,
    }

    impl JobCanceller for RecordingCanceller {
        fn request_cancel(&self, id: &JobId) -> bool {
            self.calls.lock().unwrap().push(id.clone());
            self.accepts
        }
    }

    #[derive(Clone)]
    struct Stores {
        libraries: MockLibraryRepo,
        users: MockUserRepo,
        jobs: MockJobStore,
        catalog: MockCatalogRepo,
        user_data: MockUserDataStore,
        canceller: RecordingCanceller,
    }

    impl Stores {
        fn svc(&self) -> Svc {
            MaintenanceServiceImpl::new(
                self.libraries.clone(),
                self.users.clone(),
                self.jobs.clone(),
                self.catalog.clone(),
                self.user_data.clone(),
                Arc::new(self.canceller.clone()),
            )
        }

        fn queue(&self, kind: JobKind, payload: String, status: JobStatus) -> Job {
            let mut job =
                queued_job(kind, JobPriority::Normal, payload, None, Timestamp::UNIX_EPOCH);
            job.status = status;
            self.jobs.seed(job.clone());
            job
        }

        async fn status(&self, id: &str) -> Option<ScanStatus> {
            let state = self.libraries.scan_state(&LibraryId(id.into())).await.unwrap();
            state.map(|state| state.status)
        }
    }

    fn library(id: &str, kind: LibraryKind) -> Library {
        Library {
            id: LibraryId(id.into()),
            name: id.into(),
            origin: LibraryOrigin::Local,
            kind,
            roots: vec!["/m".into()],
            sort_articles: Vec::new(),
            watcher: WatcherStrategy::Manual,
            scan_schedule: None,
            metadata_sources: Vec::new(),
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
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
            active: true,
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn movie(id: &str) -> Movie {
        Movie {
            id: MovieId(id.into()),
            title: id.into(),
            sort_title: id.into(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    fn version(id: &str, movie: &str, library: &str) -> Version {
        Version {
            id: VersionId(id.into()),
            title: TitleId::Movie(MovieId(movie.into())),
            library: LibraryId(library.into()),
            quality: Quality::Hd,
            container: "mkv".into(),
            path: format!("/m/{id}.mkv"),
            size_bytes: 10,
            duration_ms: 1000,
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn unmatched(id: &str, library: &str) -> UnmatchedFile {
        UnmatchedFile {
            id: UnmatchedFileId(id.into()),
            library: LibraryId(library.into()),
            path: format!("/m/{id}.mkv"),
            candidates: Vec::new(),
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn admin() -> Principal {
        Principal {
            user: UserId("admin".into()),
            role: Role::Admin,
            account_admin: true,
            device: None,
            age_limit: None,
        }
    }

    fn member() -> Principal {
        Principal {
            user: UserId("u1".into()),
            role: Role::User,
            account_admin: false,
            device: None,
            age_limit: None,
        }
    }

    fn lib() -> LibraryId {
        LibraryId("lib".into())
    }

    fn stores() -> Stores {
        let stores = Stores {
            libraries: MockLibraryRepo::new(),
            users: MockUserRepo::new(),
            jobs: MockJobStore::new(),
            catalog: MockCatalogRepo::new(),
            user_data: MockUserDataStore::new(),
            canceller: RecordingCanceller { accepts: true, calls: Arc::default() },
        };
        stores.libraries.insert_library(library("lib", LibraryKind::Movie));
        stores.libraries.insert_library(library("other", LibraryKind::Movie));
        stores.libraries.add_unmatched(unmatched("u-own", "lib"));
        stores.libraries.add_unmatched(unmatched("u-other", "other"));
        stores.users.insert(user("u1"));
        stores.users.insert(user("u2"));
        stores.catalog.add_movie(movie("m-own"));
        stores.catalog.add_movie(movie("m-other"));
        stores.catalog.add_version(version("v-own", "m-own", "lib"));
        stores.catalog.add_version(version("v-other", "m-other", "other"));
        stores
    }

    fn held(status: ScanStatus) -> ScanState {
        ScanState {
            library: lib(),
            status,
            progress: 0.0,
            started_at: None,
            last_scanned_at: Some(Timestamp::UNIX_EPOCH),
            error: None,
        }
    }

    fn scan_of(library: &str) -> String {
        ScanJobPayload { library: LibraryId(library.into()), mode: ScanMode::Normal }.encode()
    }

    fn subtitles_of(version: &str) -> String {
        SubtitleJobPayload {
            version_id: VersionId(version.into()),
            imdb_id: None,
            title: None,
            languages: vec!["en".into()],
            season: None,
            episode: None,
            transcribe_on_miss: false,
        }
        .encode()
    }

    fn trickplay_of(version: &str) -> String {
        TrickplayJobPayload {
            version_id: VersionId(version.into()),
            source_path: format!("/m/{version}.mkv"),
            duration_ms: 1000,
        }
        .encode()
    }

    fn relink_of(version: &str, library: &str) -> String {
        RelinkJobPayload {
            version: VersionId(version.into()),
            library: LibraryId(library.into()),
            path: format!("/m/{version}.mkv"),
            target: domain::library::ResolveTarget::Existing(TitleId::Movie(MovieId("x".into()))),
        }
        .encode()
    }

    #[tokio::test]
    async fn only_an_admin_purges_or_cancels_and_only_a_library_that_exists() {
        let stores = stores();
        let svc = stores.svc();

        assert!(matches!(
            svc.purge_library(&member(), &lib(), false).await,
            Err(MaintenanceError::Forbidden)
        ));
        assert!(matches!(
            svc.cancel_library_jobs(&member(), &lib(), CancelScope::All).await,
            Err(MaintenanceError::Forbidden)
        ));
        let missing = LibraryId("missing".into());
        assert!(matches!(
            svc.purge_library(&admin(), &missing, false).await,
            Err(MaintenanceError::NotFound)
        ));
        assert!(matches!(
            svc.cancel_library_jobs(&admin(), &missing, CancelScope::All).await,
            Err(MaintenanceError::NotFound)
        ));
        assert!(stores.catalog.get_movie(&MovieId("m-own".into())).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn a_purge_takes_the_librarys_titles_and_scan_data_and_releases_it() {
        let stores = stores();
        stores.libraries.save_scan_state(held(ScanStatus::Idle)).await.unwrap();

        let report = stores.svc().purge_library(&admin(), &lib(), false).await.unwrap();

        assert_eq!(report, PurgeReport { movies: 1, ..PurgeReport::default() });
        assert!(stores.catalog.get_movie(&MovieId("m-own".into())).await.unwrap().is_none());
        assert!(stores.catalog.get_movie(&MovieId("m-other".into())).await.unwrap().is_some());
        assert!(stores.catalog.get_version(&VersionId("v-own".into())).await.unwrap().is_some());
        assert_eq!(stores.status("lib").await, None, "the scan state goes with the purge");
        let unmatched = async |id: &str| {
            stores.libraries.get_unmatched(&UnmatchedFileId(id.into())).await.unwrap()
        };
        assert!(unmatched("u-own").await.is_none());
        assert!(unmatched("u-other").await.is_some());
        assert!(stores.user_data.forgotten().is_empty(), "user data stays unless asked");
    }

    #[tokio::test]
    async fn wiping_user_data_forgets_the_purged_titles_for_every_user() {
        let stores = stores();

        let report = stores.svc().purge_library(&admin(), &lib(), true).await.unwrap();

        assert_eq!(report.wiped_users, 2);
        let forgotten = stores.user_data.forgotten();
        let users: HashSet<String> = forgotten.iter().map(|(user, _, _)| user.0.clone()).collect();
        assert_eq!(users, HashSet::from(["u1".to_owned(), "u2".to_owned()]));
        for (_, titles, versions) in forgotten {
            assert_eq!(titles.movies, HashSet::from([MovieId("m-own".into())]));
            assert_eq!(versions, HashSet::from([VersionId("v-own".into())]));
        }
    }

    #[tokio::test]
    async fn a_job_that_would_write_what_the_purge_removes_refuses_it() {
        let blocking = [
            (JobKind::LibraryScan, scan_of("lib")),
            (
                JobKind::Ingest,
                IngestJobPayload {
                    library: lib(),
                    unmatched: UnmatchedFileId("u-own".into()),
                    path: "/m/u-own.mkv".into(),
                    target: domain::library::ResolveTarget::Existing(TitleId::Movie(MovieId(
                        "m-own".into(),
                    ))),
                }
                .encode(),
            ),
            (JobKind::Relink, relink_of("v-own", "lib")),
            (JobKind::Subtitles, subtitles_of("v-own")),
            (
                JobKind::Metadata,
                MetadataJobPayload::Title {
                    title: domain::catalog::TitleRef::Movie(MovieId("m-own".into())),
                    external_id: None,
                    force: false,
                }
                .encode(),
            ),
            (
                JobKind::Artwork,
                ArtworkJobPayload {
                    owner: domain::catalog::ArtworkOwner::Movie(MovieId("m-own".into())),
                    items: Vec::new(),
                }
                .encode(),
            ),
            (JobKind::SearchReindex, String::new()),
            (JobKind::Combine, "{".to_owned()),
        ];
        for (kind, payload) in blocking {
            for status in [JobStatus::Queued, JobStatus::Running] {
                let stores = stores();
                stores.libraries.save_scan_state(held(ScanStatus::Failed)).await.unwrap();
                let job = stores.queue(kind, payload.clone(), status);

                let refused = stores.svc().purge_library(&admin(), &lib(), true).await;

                let Err(MaintenanceError::Busy(jobs)) = refused else {
                    panic!("{kind:?} {status:?} should refuse the purge: {refused:?}");
                };
                assert_eq!(jobs.iter().map(|job| &job.id).collect::<Vec<_>>(), [&job.id]);
                let kept = stores.catalog.get_movie(&MovieId("m-own".into())).await.unwrap();
                assert!(kept.is_some());
                let restored = stores.libraries.scan_state(&lib()).await.unwrap().unwrap();
                assert_eq!(restored.status, ScanStatus::Failed, "{kind:?}");
                assert_eq!(restored.last_scanned_at, Some(Timestamp::UNIX_EPOCH));
                assert!(stores.user_data.forgotten().is_empty());
            }
        }
    }

    #[tokio::test]
    async fn work_elsewhere_or_on_what_the_purge_keeps_does_not_refuse_it() {
        let stores = stores();
        stores.queue(JobKind::LibraryScan, scan_of("other"), JobStatus::Queued);
        stores.queue(JobKind::Subtitles, subtitles_of("v-other"), JobStatus::Running);
        stores.queue(JobKind::Trickplay, trickplay_of("v-own"), JobStatus::Running);
        stores.queue(
            JobKind::Metadata,
            MetadataJobPayload::People { ids: Vec::new(), force: false }.encode(),
            JobStatus::Queued,
        );
        stores.queue(JobKind::ScheduledScan, String::new(), JobStatus::Queued);
        stores.queue(JobKind::LibraryScan, scan_of("lib"), JobStatus::Succeeded);

        let report = stores.svc().purge_library(&admin(), &lib(), false).await.unwrap();

        assert_eq!(report.movies, 1);
    }

    fn is_held_by(err: &MaintenanceError, status: ScanStatus, scan: Option<&Job>) -> bool {
        match (status, scan) {
            (ScanStatus::Purging, _) => matches!(err, MaintenanceError::Purging),
            (ScanStatus::Cleaning, _) => matches!(err, MaintenanceError::Cleaning),
            (_, None) => matches!(err, MaintenanceError::Scanning),
            (_, Some(scan)) => matches!(
                err,
                MaintenanceError::Busy(jobs) if jobs.iter().map(|job| &job.id).eq([&scan.id])
            ),
        }
    }

    #[tokio::test]
    async fn a_library_already_held_refuses_a_purge_by_what_holds_it_and_is_left_alone() {
        let holds = [
            (ScanStatus::Queued, true),
            (ScanStatus::Running, true),
            (ScanStatus::Running, false),
            (ScanStatus::Purging, true),
            (ScanStatus::Cleaning, false),
        ];
        for (status, with_job) in holds {
            let stores = stores();
            stores.libraries.save_scan_state(held(status)).await.unwrap();
            let scan = with_job
                .then(|| stores.queue(JobKind::LibraryScan, scan_of("lib"), JobStatus::Queued));

            let refused = stores.svc().purge_library(&admin(), &lib(), false).await.unwrap_err();

            assert!(is_held_by(&refused, status, scan.as_ref()), "{status:?}: {refused:?}");
            assert_eq!(stores.status("lib").await, Some(status));
            assert!(stores.catalog.get_movie(&MovieId("m-own".into())).await.unwrap().is_some());
        }
    }

    #[tokio::test]
    async fn a_purge_or_clean_hold_answers_without_reading_the_jobs() {
        for status in [ScanStatus::Purging, ScanStatus::Cleaning] {
            let stores = stores();
            stores.libraries.save_scan_state(held(status)).await.unwrap();
            stores.jobs.set_fail();
            let svc = stores.svc();

            let refused = [
                svc.purge_library(&admin(), &lib(), false).await.unwrap_err(),
                svc.clean_library(&admin(), &lib(), false).await.unwrap_err(),
                svc.clean_all(&admin(), false).await.unwrap_err(),
            ];

            for err in refused {
                assert!(is_held_by(&err, status, None), "{status:?}: {err:?}");
            }
        }
    }

    #[tokio::test]
    async fn a_purge_that_fails_part_way_gives_the_library_back() {
        let failing_catalog = stores();
        failing_catalog.catalog.set_fail();
        let failing_wipe = stores();
        failing_wipe.user_data.set_fail_forget();
        let failing_jobs = stores();
        failing_jobs.jobs.set_fail();
        for stores in [failing_catalog, failing_wipe, failing_jobs] {
            let failed = stores.svc().purge_library(&admin(), &lib(), true).await;

            assert!(matches!(failed, Err(MaintenanceError::Repository(_))), "{failed:?}");
            assert_eq!(stores.status("lib").await, Some(ScanStatus::Idle));
        }
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn a_library_that_cannot_be_given_back_says_so() {
        let stores = stores();
        stores.libraries.fail_saves_after(1);

        let failed = stores.svc().purge_library(&admin(), &lib(), false).await;

        assert!(matches!(failed, Err(MaintenanceError::Repository(_))));
        assert_eq!(stores.status("lib").await, Some(ScanStatus::Purging));
        assert!(logs_contain("releasing library [lib] after a purge that did not finish failed"));
    }

    fn gone(id: &str, movie: &str, library: &str) -> Version {
        Version { available: false, ..version(id, movie, library) }
    }

    fn with_leftovers() -> Stores {
        let stores = stores();
        for id in ["m-gone", "m-other-gone", "m-empty"] {
            stores.catalog.add_movie(movie(id));
        }
        stores.catalog.add_version(gone("v-gone", "m-gone", "lib"));
        stores.catalog.add_version(gone("v-other-gone", "m-other-gone", "other"));
        stores.catalog.add_version(gone("v-deleted", "m-own", "deleted"));
        stores
    }

    async fn has_movie(stores: &Stores, id: &str) -> bool {
        stores.catalog.get_movie(&MovieId(id.into())).await.unwrap().is_some()
    }

    #[tokio::test]
    async fn only_an_admin_cleans_and_only_a_library_that_exists() {
        let stores = with_leftovers();
        let svc = stores.svc();

        assert!(matches!(
            svc.clean_library(&member(), &lib(), false).await,
            Err(MaintenanceError::Forbidden)
        ));
        assert!(matches!(svc.clean_all(&member(), false).await, Err(MaintenanceError::Forbidden)));
        assert!(matches!(
            svc.clean_library(&admin(), &LibraryId("missing".into()), false).await,
            Err(MaintenanceError::NotFound)
        ));
        assert!(has_movie(&stores, "m-gone").await);
    }

    #[tokio::test]
    async fn a_clean_takes_the_librarys_missing_files_and_the_titles_they_leave_empty() {
        let stores = with_leftovers();
        stores.libraries.save_scan_state(held(ScanStatus::Failed)).await.unwrap();

        let report = stores.svc().clean_library(&admin(), &lib(), false).await.unwrap();

        assert_eq!(report, CleanReport { versions: 1, movies: 1, ..CleanReport::default() });
        assert!(!has_movie(&stores, "m-gone").await);
        for id in ["m-own", "m-other-gone", "m-empty"] {
            assert!(has_movie(&stores, id).await, "{id}");
        }
        let restored = stores.libraries.scan_state(&lib()).await.unwrap().unwrap();
        assert_eq!(restored.status, ScanStatus::Failed, "the library is given back as it was");
        assert_eq!(restored.last_scanned_at, Some(Timestamp::UNIX_EPOCH));
        assert!(stores.user_data.forgotten().is_empty(), "user data stays unless asked");
    }

    #[tokio::test]
    async fn cleaning_every_library_takes_every_missing_file_and_every_empty_title() {
        let stores = with_leftovers();
        stores.libraries.save_scan_state(held(ScanStatus::Idle)).await.unwrap();

        let report = stores.svc().clean_all(&admin(), false).await.unwrap();

        assert_eq!(report, CleanReport { versions: 3, movies: 3, ..CleanReport::default() });
        for id in ["m-gone", "m-other-gone", "m-empty"] {
            assert!(!has_movie(&stores, id).await, "{id}");
        }
        assert!(has_movie(&stores, "m-own").await, "a title with a version left stays");
        for id in ["lib", "other"] {
            assert_eq!(stores.status(id).await, Some(ScanStatus::Idle), "{id}");
        }
    }

    #[tokio::test]
    async fn wiping_user_data_forgets_what_the_clean_removed_for_every_user() {
        let stores = with_leftovers();

        let report = stores.svc().clean_library(&admin(), &lib(), true).await.unwrap();

        assert_eq!(report.wiped_users, 2);
        let forgotten = stores.user_data.forgotten();
        assert_eq!(forgotten.len(), 2);
        for (_, titles, versions) in forgotten {
            assert_eq!(titles.movies, HashSet::from([MovieId("m-gone".into())]));
            assert_eq!(versions, HashSet::from([VersionId("v-gone".into())]));
        }
    }

    #[tokio::test]
    async fn a_wipe_with_nothing_removed_forgets_nothing_and_counts_no_user() {
        let stores = stores();
        stores.libraries.insert_library(library("empty", LibraryKind::Movie));
        let svc = stores.svc();

        let purged = svc.purge_library(&admin(), &LibraryId("empty".into()), true).await;
        let cleaned = svc.clean_library(&admin(), &lib(), true).await;
        let every = svc.clean_all(&admin(), true).await;

        assert_eq!(purged.unwrap(), PurgeReport::default());
        assert_eq!(cleaned.unwrap(), CleanReport::default());
        assert_eq!(every.unwrap(), CleanReport::default());
        assert!(stores.user_data.forgotten().is_empty());
    }

    #[tokio::test]
    async fn work_that_writes_titles_refuses_a_clean_and_work_on_a_file_does_not() {
        let blocking = [
            (JobKind::LibraryScan, scan_of("lib")),
            (JobKind::Ingest, "{".to_owned()),
            (JobKind::Relink, relink_of("v-own", "lib")),
            (
                JobKind::Metadata,
                MetadataJobPayload::Title {
                    title: domain::catalog::TitleRef::Movie(MovieId("m-own".into())),
                    external_id: None,
                    force: false,
                }
                .encode(),
            ),
            (
                JobKind::Artwork,
                ArtworkJobPayload {
                    owner: domain::catalog::ArtworkOwner::Movie(MovieId("m-own".into())),
                    items: Vec::new(),
                }
                .encode(),
            ),
            (JobKind::Dedup, String::new()),
            (JobKind::SearchReindex, String::new()),
        ];
        for (kind, payload) in blocking {
            let stores = with_leftovers();
            stores.libraries.save_scan_state(held(ScanStatus::Failed)).await.unwrap();
            let job = stores.queue(kind, payload, JobStatus::Running);

            let refused = stores.svc().clean_library(&admin(), &lib(), true).await;

            let Err(MaintenanceError::Busy(jobs)) = refused else {
                panic!("{kind:?} should refuse the clean: {refused:?}");
            };
            assert_eq!(jobs.iter().map(|job| &job.id).collect::<Vec<_>>(), [&job.id]);
            assert!(has_movie(&stores, "m-gone").await);
            assert_eq!(stores.status("lib").await, Some(ScanStatus::Failed), "{kind:?}");
            assert!(stores.user_data.forgotten().is_empty());
        }

        let stores = with_leftovers();
        stores.queue(JobKind::Subtitles, subtitles_of("v-gone"), JobStatus::Running);
        stores.queue(JobKind::Trickplay, trickplay_of("v-own"), JobStatus::Queued);
        stores.queue(JobKind::LibraryScan, scan_of("other"), JobStatus::Queued);
        let report = stores.svc().clean_library(&admin(), &lib(), false).await.unwrap();
        assert_eq!(report.versions, 1);
    }

    #[tokio::test]
    async fn cleaning_every_library_waits_for_work_on_any_of_them_but_not_on_a_file() {
        let stores = with_leftovers();
        let scan = stores.queue(JobKind::LibraryScan, scan_of("other"), JobStatus::Queued);
        stores.queue(JobKind::Subtitles, subtitles_of("v-other-gone"), JobStatus::Running);
        stores.queue(
            JobKind::Artwork,
            ArtworkJobPayload {
                owner: domain::catalog::ArtworkOwner::Person(domain::metadata::PersonId(
                    "p".into(),
                )),
                items: Vec::new(),
            }
            .encode(),
            JobStatus::Queued,
        );

        let refused = stores.svc().clean_all(&admin(), false).await;

        let Err(MaintenanceError::Busy(jobs)) = refused else {
            panic!("a scan of another library should refuse it: {refused:?}");
        };
        let named: Vec<JobId> = jobs.into_iter().map(|job| job.id).collect();
        assert_eq!(named, std::slice::from_ref(&scan.id));
        assert!(has_movie(&stores, "m-empty").await);
        stores.jobs.seed(Job { status: JobStatus::Succeeded, ..scan });
        assert_eq!(stores.svc().clean_all(&admin(), false).await.unwrap().movies, 3);
    }

    #[tokio::test]
    async fn a_library_already_held_refuses_either_clean_by_what_holds_it_and_nothing_is_taken() {
        let holds = [
            (ScanStatus::Running, true),
            (ScanStatus::Queued, false),
            (ScanStatus::Purging, false),
            (ScanStatus::Cleaning, true),
        ];
        for (status, with_job) in holds {
            let stores = with_leftovers();
            stores.libraries.save_scan_state(held(status)).await.unwrap();
            let scan = with_job
                .then(|| stores.queue(JobKind::LibraryScan, scan_of("lib"), JobStatus::Queued));
            let svc = stores.svc();

            let one = svc.clean_library(&admin(), &lib(), false).await.unwrap_err();
            let every = svc.clean_all(&admin(), false).await.unwrap_err();

            assert!(is_held_by(&one, status, scan.as_ref()), "{status:?}: {one:?}");
            assert!(is_held_by(&every, status, scan.as_ref()), "{status:?}: {every:?}");
            assert_eq!(stores.status("lib").await, Some(status));
            assert_eq!(stores.status("other").await, None, "no library is held");
            assert!(has_movie(&stores, "m-gone").await);
        }
    }

    #[tokio::test]
    async fn a_clean_that_fails_part_way_gives_every_library_back() {
        let failing_catalog = with_leftovers();
        failing_catalog.catalog.set_fail();
        let failing_wipe = with_leftovers();
        failing_wipe.user_data.set_fail_forget();
        let failing_jobs = with_leftovers();
        failing_jobs.jobs.set_fail();
        for stores in [failing_catalog, failing_wipe, failing_jobs] {
            let svc = stores.svc();
            let one = svc.clean_library(&admin(), &lib(), true).await;
            let every = svc.clean_all(&admin(), true).await;

            assert!(matches!(one, Err(MaintenanceError::Repository(_))), "{one:?}");
            assert!(matches!(every, Err(MaintenanceError::Repository(_))), "{every:?}");
            for id in ["lib", "other"] {
                assert_eq!(stores.status(id).await, Some(ScanStatus::Idle), "{id}");
            }
        }
        let unlisted = with_leftovers();
        unlisted.libraries.set_fail_list();
        let failed = unlisted.svc().clean_all(&admin(), false).await;
        assert!(matches!(failed, Err(MaintenanceError::Repository(_))));
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn a_library_that_cannot_be_held_or_given_back_after_a_clean_says_so() {
        let stores = with_leftovers();
        stores.libraries.fail_saves_after(1);

        let failed = stores.svc().clean_all(&admin(), false).await;

        assert!(matches!(failed, Err(MaintenanceError::Repository(_))), "{failed:?}");
        assert!(has_movie(&stores, "m-gone").await, "nothing is cleaned without every hold");
        assert!(logs_contain("after a clean failed"));

        let stores = with_leftovers();
        stores.libraries.fail_saves_after(1);
        let cleaned = stores.svc().clean_library(&admin(), &lib(), false).await.unwrap();
        assert_eq!(cleaned.versions, 1);
        assert_eq!(stores.status("lib").await, Some(ScanStatus::Cleaning));
        assert!(logs_contain("releasing library [lib] after a clean failed"));
    }

    #[tokio::test]
    async fn cancelling_stops_the_librarys_jobs_and_nothing_else() {
        let stores = stores();
        stores.libraries.save_scan_state(held(ScanStatus::Queued)).await.unwrap();
        let queued_scan = stores.queue(JobKind::LibraryScan, scan_of("lib"), JobStatus::Queued);
        let queued_search =
            stores.queue(JobKind::Subtitles, subtitles_of("v-own"), JobStatus::Queued);
        let running_trickplay =
            stores.queue(JobKind::Trickplay, trickplay_of("v-own"), JobStatus::Running);
        let running_relink =
            stores.queue(JobKind::Relink, relink_of("v-own", "lib"), JobStatus::Running);
        let elsewhere =
            stores.queue(JobKind::Subtitles, subtitles_of("v-other"), JobStatus::Queued);
        let reindex = stores.queue(JobKind::SearchReindex, String::new(), JobStatus::Queued);

        let outcome =
            stores.svc().cancel_library_jobs(&admin(), &lib(), CancelScope::All).await.unwrap();

        assert_eq!(outcome, CancelledJobs { cancelled: 2, signalled: 1, running: 1 });
        let signalled = stores.canceller.calls.lock().unwrap().clone();
        assert_eq!(signalled, std::slice::from_ref(&running_trickplay.id));
        for (job, status) in [
            (&queued_scan, JobStatus::Cancelled),
            (&queued_search, JobStatus::Cancelled),
            (&running_trickplay, JobStatus::Running),
            (&running_relink, JobStatus::Running),
            (&elsewhere, JobStatus::Queued),
            (&reindex, JobStatus::Queued),
        ] {
            let stored = stores.jobs.get(&job.id).await.unwrap().unwrap();
            assert_eq!(stored.status, status, "{:?}", job.kind);
        }
        let freed = stores.status("lib").await;
        assert_eq!(freed, Some(ScanStatus::Idle), "a cancelled scan frees its slot");
        stores.jobs.seed(Job { status: JobStatus::Succeeded, ..reindex });
        let refused = stores.svc().purge_library(&admin(), &lib(), false).await;
        let Err(MaintenanceError::Busy(left)) = refused else {
            panic!("the relink still running should refuse the purge: {refused:?}");
        };
        assert_eq!(left.into_iter().map(|job| job.id).collect::<Vec<_>>(), [running_relink.id]);
    }

    #[tokio::test]
    async fn a_running_job_that_cannot_be_signalled_is_counted_as_left_running() {
        let mut stores = stores();
        stores.canceller.accepts = false;
        stores.queue(JobKind::Trickplay, trickplay_of("v-own"), JobStatus::Running);

        let outcome =
            stores.svc().cancel_library_jobs(&admin(), &lib(), CancelScope::All).await.unwrap();

        assert_eq!(outcome, CancelledJobs { cancelled: 0, signalled: 0, running: 1 });
    }

    #[tokio::test]
    async fn cancelling_for_a_purge_or_a_clean_stops_only_the_librarys_jobs_that_block_it() {
        let scopes = [
            (CancelScope::PurgeBlockers, [true, true, false, false]),
            (CancelScope::CleanBlockers, [true, false, false, false]),
        ];
        for (scope, cancelled) in scopes {
            let stores = stores();
            let jobs = [
                stores.queue(JobKind::LibraryScan, scan_of("lib"), JobStatus::Queued),
                stores.queue(JobKind::Subtitles, subtitles_of("v-own"), JobStatus::Queued),
                stores.queue(JobKind::Trickplay, trickplay_of("v-own"), JobStatus::Queued),
                stores.queue(JobKind::SearchReindex, String::new(), JobStatus::Queued),
            ];

            let outcome = stores.svc().cancel_library_jobs(&admin(), &lib(), scope).await.unwrap();

            let count = cancelled.iter().filter(|gone| **gone).count() as u64;
            assert_eq!(outcome, CancelledJobs { cancelled: count, signalled: 0, running: 0 });
            for (job, gone) in jobs.iter().zip(cancelled) {
                let status = stores.jobs.get(&job.id).await.unwrap().unwrap().status;
                let expected = if gone { JobStatus::Cancelled } else { JobStatus::Queued };
                assert_eq!(status, expected, "{scope:?}: {:?}", job.kind);
            }
        }
    }

    #[tokio::test]
    async fn cancelling_reports_a_job_store_that_cannot_list() {
        let stores = stores();
        stores.jobs.set_fail();

        let failed = stores.svc().cancel_library_jobs(&admin(), &lib(), CancelScope::All).await;

        assert!(matches!(failed, Err(MaintenanceError::Repository(_))));
    }

    fn episode_file(path: &str) -> DiscoveredFile {
        DiscoveredFile {
            library: lib(),
            path: path.into(),
            size_bytes: 10,
            subtitle_siblings: Vec::new(),
            probe: ProbeResult {
                duration_ms: 1000,
                video: Vec::new(),
                audio: Vec::new(),
                subtitles: Vec::new(),
                chapters: Vec::new(),
            },
        }
    }

    fn trickplay(version: &VersionId) -> TrickplayAsset {
        TrickplayAsset {
            version: version.clone(),
            interval_ms: 5_000,
            columns: 8,
            rows: 8,
            tile_width: 320,
            tile_height: 180,
            sheet_paths: vec![format!("/tp/{}/sheet-001.jpg", version.0)],
        }
    }

    async fn queued(jobs: &MockJobStore, kind: JobKind) -> usize {
        let all = jobs.list().await.unwrap();
        all.iter().filter(|job| job.kind == kind && job.status == JobStatus::Queued).count()
    }

    #[tokio::test]
    async fn a_movie_library_over_episodes_is_purged_retyped_and_scanned_into_episodes() {
        let stores = stores();
        let paths = ["/m/Show/Show S01E01.mkv", "/m/Show/Show S01E02.mkv"];
        let report = ScanReport {
            discovered: paths.iter().map(|path| episode_file(path)).collect(),
            skipped: Vec::new(),
            total_candidates: paths.len(),
            offline: Vec::new(),
        };
        let enricher = Enricher::new(
            stores.catalog.clone(),
            None::<MockMetadataProvider>,
            stores.jobs.clone(),
            stores.libraries.clone(),
        )
        .with_subtitle_languages(vec!["en".into()]);
        let versions: Vec<VersionId> =
            paths.iter().map(|path| VersionId(derive_id("version", path))).collect();

        enricher.enrich(&library("lib", LibraryKind::Movie), &report, None).await;
        let wrong = stores.catalog.get_version(&versions[0]).await.unwrap().unwrap();
        assert!(matches!(wrong.title, TitleId::Movie(_)), "both files land on one movie");
        for version in &versions {
            stores.catalog.set_trickplay(version, &[trickplay(version)]).await.unwrap();
        }

        let svc = stores.svc();
        let refused = svc.purge_library(&admin(), &lib(), false).await;
        assert!(matches!(refused, Err(MaintenanceError::Busy(_))), "the first scan's searches");
        svc.cancel_library_jobs(&admin(), &lib(), CancelScope::PurgeBlockers).await.unwrap();
        let purged = svc.purge_library(&admin(), &lib(), false).await.unwrap();
        assert_eq!(purged.movies, 2, "the show's movie and m-own");
        stores.libraries.upsert(library("lib", LibraryKind::Tv)).await.unwrap();
        enricher.enrich(&library("lib", LibraryKind::Tv), &report, None).await;

        for version in &versions {
            let detail = stores.catalog.version_detail(version).await.unwrap().unwrap();
            let TitleId::Episode(episode) = &detail.version.title else {
                panic!("{} should be on an episode: {:?}", version.0, detail.version.title);
            };
            assert!(stores.catalog.get_episode(episode).await.unwrap().is_some());
            assert_eq!(detail.trickplay.len(), 1, "trickplay belongs to the file and stays");
        }
        let kept = "the first scan's, left by the cancel; the scan queues none";
        assert_eq!(queued(&stores.jobs, JobKind::Trickplay).await, 2, "{kept}");
        assert_eq!(queued(&stores.jobs, JobKind::PlaybackPrep).await, 2, "{kept}");
        assert_eq!(queued(&stores.jobs, JobKind::Subtitles).await, 2, "searched again by episode");
    }
}

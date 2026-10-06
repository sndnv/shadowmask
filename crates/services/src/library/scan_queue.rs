use std::collections::HashSet;

use domain::common::PageRequest;
use domain::error::RepositoryError;
use domain::job::{JobId, JobKind, JobPriority, JobQuery};
use domain::library::{
    Library, LibraryId, LibraryOrigin, ScanMode, ScanState, ScanStatus, WatcherStrategy,
};
use domain::repository::{JobRepository, LibraryRepository};
use jiff::Timestamp;

use super::ScanJobPayload;
use crate::job::queued_job;

pub fn is_nightly_library(library: &Library) -> bool {
    library.watcher == WatcherStrategy::Scheduled && library.origin == LibraryOrigin::Local
}

async fn reserve_scan<L>(libraries: &L, id: &LibraryId) -> Result<bool, RepositoryError>
where
    L: LibraryRepository + Sync,
{
    if matches!(
        libraries.scan_state(id).await?,
        Some(state) if matches!(state.status, ScanStatus::Queued | ScanStatus::Running)
    ) {
        return Ok(false);
    }
    libraries
        .save_scan_state(ScanState {
            library: id.clone(),
            status: ScanStatus::Queued,
            progress: 0.0,
            started_at: None,
            last_scanned_at: None,
            error: None,
        })
        .await?;
    Ok(true)
}

async fn release_scan<L>(libraries: &L, id: &LibraryId) -> Result<(), RepositoryError>
where
    L: LibraryRepository + Sync,
{
    if !matches!(libraries.scan_state(id).await?, Some(state) if state.status == ScanStatus::Queued)
    {
        return Ok(());
    }
    libraries
        .save_scan_state(ScanState {
            library: id.clone(),
            status: ScanStatus::Idle,
            progress: 0.0,
            started_at: None,
            last_scanned_at: None,
            error: None,
        })
        .await
}

async fn enqueue_scan<J>(
    jobs: &J,
    id: &LibraryId,
    mode: ScanMode,
    parent: Option<&JobId>,
) -> Result<(), RepositoryError>
where
    J: JobRepository + Sync,
{
    let now = Timestamp::now();
    let payload = ScanJobPayload { library: id.clone(), mode }.encode();
    jobs.enqueue(queued_job(JobKind::LibraryScan, JobPriority::Normal, payload, parent, now)).await
}

pub async fn queue_scan<L, J>(
    libraries: &L,
    jobs: &J,
    id: &LibraryId,
    mode: ScanMode,
    parent: Option<&JobId>,
) -> Result<bool, RepositoryError>
where
    L: LibraryRepository + Sync,
    J: JobRepository + Sync,
{
    if !reserve_scan(libraries, id).await? {
        return Ok(false);
    }
    if let Err(err) = enqueue_scan(jobs, id, mode, parent).await {
        if let Err(release) = release_scan(libraries, id).await {
            tracing::warn!("releasing the scan slot of library [{}] failed: [{release}]", id.0);
        }
        return Err(err);
    }
    Ok(true)
}

pub async fn settle_interrupted_scans<L, J>(
    libraries: &L,
    jobs: &J,
) -> Result<usize, RepositoryError>
where
    L: LibraryRepository + Sync,
    J: JobRepository + Sync,
{
    let pending: HashSet<String> = jobs
        .list_page(&JobQuery::active(true), PageRequest::ALL)
        .await?
        .into_iter()
        .filter(|job| job.kind == JobKind::LibraryScan)
        .map(|job| ScanJobPayload::decode(&job.payload).library.0)
        .collect();
    let mut settled = 0;
    for library in libraries.list().await? {
        let Some(state) = libraries.scan_state(&library.id).await? else {
            continue;
        };
        let pending_scan = pending.contains(&library.id.0);
        match state.status {
            ScanStatus::Running if pending_scan => {
                libraries
                    .save_scan_state(ScanState {
                        status: ScanStatus::Queued,
                        progress: 0.0,
                        error: None,
                        ..state
                    })
                    .await?;
            }
            ScanStatus::Running => {
                libraries
                    .save_scan_state(ScanState {
                        status: ScanStatus::Failed,
                        progress: 0.0,
                        error: Some("interrupted by a server restart".into()),
                        ..state
                    })
                    .await?;
            }
            ScanStatus::Queued if !pending_scan => {
                release_scan(libraries, &library.id).await?;
            }
            _ => continue,
        }
        settled += 1;
    }
    Ok(settled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::library::LibraryKind;
    use mocks::{MockJobStore, MockLibraryRepo};

    fn library(id: &str) -> Library {
        Library {
            id: LibraryId(id.into()),
            name: id.into(),
            origin: LibraryOrigin::Local,
            kind: LibraryKind::Movie,
            roots: vec!["/m".into()],
            sort_articles: Vec::new(),
            watcher: WatcherStrategy::Manual,
            scan_schedule: None,
            metadata_sources: Vec::new(),
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn state(id: &str, status: ScanStatus) -> ScanState {
        ScanState {
            library: LibraryId(id.into()),
            status,
            progress: 0.5,
            started_at: Some(Timestamp::UNIX_EPOCH),
            last_scanned_at: None,
            error: None,
        }
    }

    #[tokio::test]
    async fn a_restart_settles_every_scan_state_no_job_will_finish() {
        let libraries = MockLibraryRepo::new();
        let jobs = MockJobStore::new();
        for id in ["running", "requeued", "idle", "queued", "orphaned", "never"] {
            libraries.insert_library(library(id));
        }
        libraries.save_scan_state(state("running", ScanStatus::Running)).await.unwrap();
        libraries.save_scan_state(state("requeued", ScanStatus::Running)).await.unwrap();
        libraries.save_scan_state(state("idle", ScanStatus::Idle)).await.unwrap();
        libraries.save_scan_state(state("queued", ScanStatus::Queued)).await.unwrap();
        libraries.save_scan_state(state("orphaned", ScanStatus::Queued)).await.unwrap();
        enqueue_scan(&jobs, &LibraryId("queued".into()), ScanMode::Normal, None).await.unwrap();
        enqueue_scan(&jobs, &LibraryId("requeued".into()), ScanMode::Reread, None).await.unwrap();

        assert_eq!(settle_interrupted_scans(&libraries, &jobs).await.unwrap(), 3);

        let requeued = LibraryId("requeued".into());
        let waiting = libraries.scan_state(&requeued).await.unwrap().unwrap();
        assert_eq!((waiting.status, waiting.progress), (ScanStatus::Queued, 0.0));
        assert!(
            !reserve_scan(&libraries, &requeued).await.unwrap(),
            "its job still holds the slot"
        );

        let running = LibraryId("running".into());
        let failed = libraries.scan_state(&running).await.unwrap().unwrap();
        assert_eq!(failed.status, ScanStatus::Failed);
        assert_eq!(failed.progress, 0.0);
        assert_eq!(failed.started_at, Some(Timestamp::UNIX_EPOCH));
        assert_eq!(failed.error.as_deref(), Some("interrupted by a server restart"));
        assert!(reserve_scan(&libraries, &running).await.unwrap());

        let orphaned = LibraryId("orphaned".into());
        assert_eq!(
            libraries.scan_state(&orphaned).await.unwrap().unwrap().status,
            ScanStatus::Idle
        );
        assert!(reserve_scan(&libraries, &orphaned).await.unwrap());

        for (id, status) in [("idle", ScanStatus::Idle), ("queued", ScanStatus::Queued)] {
            let kept = libraries.scan_state(&LibraryId(id.into())).await.unwrap().unwrap();
            assert_eq!((kept.status, kept.progress), (status, 0.5), "{id}");
        }
        assert!(libraries.scan_state(&LibraryId("never".into())).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn settling_interrupted_scans_reports_a_store_that_cannot_list() {
        let libraries = MockLibraryRepo::new();
        libraries.set_fail_list();

        settle_interrupted_scans(&libraries, &MockJobStore::new()).await.unwrap_err();
    }
    #[tokio::test]
    async fn a_reservation_holds_the_slot_before_any_job_exists() {
        let libraries = MockLibraryRepo::new();
        let jobs = MockJobStore::new();
        let id = LibraryId("lib".into());

        assert!(reserve_scan(&libraries, &id).await.unwrap());

        assert_eq!(libraries.scan_state(&id).await.unwrap().unwrap().status, ScanStatus::Queued);
        assert!(jobs.list().await.unwrap().is_empty());
        assert!(!reserve_scan(&libraries, &id).await.unwrap(), "the slot is already taken");

        enqueue_scan(&jobs, &id, ScanMode::Normal, None).await.unwrap();

        assert_eq!(jobs.list().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn releasing_a_reservation_gives_the_slot_back() {
        let libraries = MockLibraryRepo::new();
        let id = LibraryId("lib".into());

        assert!(reserve_scan(&libraries, &id).await.unwrap());
        release_scan(&libraries, &id).await.unwrap();

        assert_eq!(libraries.scan_state(&id).await.unwrap().unwrap().status, ScanStatus::Idle);
        assert!(reserve_scan(&libraries, &id).await.unwrap(), "the slot is free again");
    }

    #[tokio::test]
    async fn releasing_leaves_a_scan_that_has_already_started_alone() {
        let libraries = MockLibraryRepo::new();
        let id = LibraryId("lib".into());
        libraries
            .save_scan_state(ScanState {
                library: id.clone(),
                status: ScanStatus::Running,
                progress: 0.5,
                started_at: Some(Timestamp::UNIX_EPOCH),
                last_scanned_at: None,
                error: None,
            })
            .await
            .unwrap();

        release_scan(&libraries, &id).await.unwrap();

        let state = libraries.scan_state(&id).await.unwrap().unwrap();
        assert_eq!(state.status, ScanStatus::Running);
        assert_eq!(state.progress, 0.5);
    }

    #[tokio::test]
    async fn releasing_a_slot_nobody_holds_writes_nothing() {
        let libraries = MockLibraryRepo::new();
        let id = LibraryId("lib".into());

        release_scan(&libraries, &id).await.unwrap();

        assert!(libraries.scan_state(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn queueing_a_scan_reserves_and_enqueues_in_one_step() {
        let libraries = MockLibraryRepo::new();
        let jobs = MockJobStore::new();
        let id = LibraryId("lib".into());

        assert!(queue_scan(&libraries, &jobs, &id, ScanMode::Normal, None).await.unwrap());
        assert!(!queue_scan(&libraries, &jobs, &id, ScanMode::Reread, None).await.unwrap());

        let queued = jobs.list().await.unwrap();
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].payload, "lib");
    }

    #[tokio::test]
    async fn a_reread_is_queued_with_its_mode() {
        let libraries = MockLibraryRepo::new();
        let jobs = MockJobStore::new();
        let id = LibraryId("lib".into());

        assert!(queue_scan(&libraries, &jobs, &id, ScanMode::Reread, None).await.unwrap());

        let queued = jobs.list().await.unwrap();
        assert_eq!(
            ScanJobPayload::decode(&queued[0].payload),
            ScanJobPayload { library: id, mode: ScanMode::Reread }
        );
    }

    #[tokio::test]
    async fn a_scan_that_cannot_be_enqueued_holds_no_slot() {
        let libraries = MockLibraryRepo::new();
        let jobs = MockJobStore::new();
        let id = LibraryId("lib".into());
        jobs.set_fail();

        queue_scan(&libraries, &jobs, &id, ScanMode::Normal, None).await.unwrap_err();

        assert_eq!(libraries.scan_state(&id).await.unwrap().unwrap().status, ScanStatus::Idle);
        assert!(reserve_scan(&libraries, &id).await.unwrap(), "the next scan can still start");
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn a_failed_release_does_not_hide_why_the_enqueue_failed() {
        let libraries = MockLibraryRepo::new();
        let jobs = MockJobStore::new();
        let id = LibraryId("lib".into());
        jobs.set_fail();
        libraries.fail_saves_after(1);

        let err = queue_scan(&libraries, &jobs, &id, ScanMode::Normal, None).await.unwrap_err();

        assert_eq!(err.to_string(), "repository backend error: mock job store failure");
        assert!(logs_contain("releasing the scan slot of library [lib] failed"));
    }
}

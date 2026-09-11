use domain::error::RepositoryError;
use domain::job::{JobId, JobKind, JobPriority};
use domain::library::{Library, LibraryId, LibraryOrigin, ScanState, ScanStatus, WatcherStrategy};
use domain::repository::{JobRepository, LibraryRepository};
use jiff::Timestamp;

use crate::job::queued_job;

pub fn is_nightly_library(library: &Library) -> bool {
    library.watcher == WatcherStrategy::Scheduled && library.origin == LibraryOrigin::Local
}

pub async fn queue_scan<L, J>(
    libraries: &L,
    jobs: &J,
    id: &LibraryId,
    parent: Option<&JobId>,
) -> Result<bool, RepositoryError>
where
    L: LibraryRepository + Sync,
    J: JobRepository + Sync,
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
    let now = Timestamp::now();
    jobs.enqueue(queued_job(JobKind::LibraryScan, JobPriority::Normal, id.0.clone(), parent, now))
        .await?;
    Ok(true)
}

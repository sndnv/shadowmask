use domain::job::Job;
use domain::repository::{JobRepository, LibraryRepository};
use services::library::{is_nightly_library, queue_scan};

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct ScheduledScanHandler<L, J> {
    libraries: L,
    jobs: J,
}

impl<L, J> ScheduledScanHandler<L, J> {
    pub fn new(libraries: L, jobs: J) -> Self {
        Self { libraries, jobs }
    }
}

impl<L, J> JobHandler for ScheduledScanHandler<L, J>
where
    L: LibraryRepository + Send + Sync,
    J: JobRepository + Send + Sync,
{
    async fn handle(&self, job: &Job) -> Result<(), JobError> {
        let libraries =
            self.libraries.list().await.map_err(|err| JobError::Retryable(err.to_string()))?;
        let mut queued = 0usize;
        let mut skipped = 0usize;
        for library in libraries.iter().filter(|l| is_nightly_library(l)) {
            let started = queue_scan(&self.libraries, &self.jobs, &library.id, Some(&job.id))
                .await
                .map_err(|err| JobError::Retryable(err.to_string()))?;
            if started {
                queued += 1;
            } else {
                skipped += 1;
            }
        }
        tracing::info!(
            "nightly scan queued [{queued}] libraries, skipped [{skipped}] already running"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use domain::library::{
        Library, LibraryId, LibraryKind, LibraryOrigin, ScanState, ScanStatus, WatcherStrategy,
    };
    use jiff::Timestamp;
    use mocks::{MockJobStore, MockLibraryRepo};

    use super::*;

    fn library(id: &str, watcher: WatcherStrategy, origin: LibraryOrigin) -> Library {
        Library {
            id: LibraryId(id.to_owned()),
            name: id.to_owned(),
            kind: LibraryKind::Movie,
            origin,
            sort_articles: Vec::new(),
            roots: vec!["/m".to_owned()],
            watcher,
            scan_schedule: None,
            metadata_sources: Vec::new(),
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn trigger() -> Job {
        Job {
            id: JobId("nightly".to_owned()),
            kind: JobKind::ScheduledScan,
            status: JobStatus::Running,
            priority: JobPriority::Normal,
            payload: String::new(),
            attempts: 0,
            progress: 0.0,
            available_at: Timestamp::UNIX_EPOCH,
            last_error: None,
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            started_at: None,
            finished_at: None,
            parent_id: None,
        }
    }

    #[tokio::test]
    async fn only_scheduled_local_libraries_are_scanned() {
        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library(
            "nightly",
            WatcherStrategy::Scheduled,
            LibraryOrigin::Local,
        ));
        libraries.insert_library(library("manual", WatcherStrategy::Manual, LibraryOrigin::Local));
        libraries.insert_library(library("watched", WatcherStrategy::Local, LibraryOrigin::Local));
        libraries.insert_library(library(
            "external",
            WatcherStrategy::Scheduled,
            LibraryOrigin::External,
        ));
        let jobs = MockJobStore::new();

        ScheduledScanHandler::new(libraries.clone(), jobs.clone())
            .handle(&trigger())
            .await
            .unwrap();

        let queued = jobs.list().await.unwrap();
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].kind, JobKind::LibraryScan);
        assert_eq!(queued[0].payload, "nightly");
        assert_eq!(queued[0].parent_id, Some(JobId("nightly".to_owned())));
        assert_eq!(
            libraries.scan_state(&LibraryId("nightly".to_owned())).await.unwrap().unwrap().status,
            ScanStatus::Queued
        );
    }

    #[tokio::test]
    async fn a_library_already_scanning_is_left_alone() {
        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library(
            "nightly",
            WatcherStrategy::Scheduled,
            LibraryOrigin::Local,
        ));
        libraries
            .save_scan_state(ScanState {
                library: LibraryId("nightly".to_owned()),
                status: ScanStatus::Running,
                progress: 0.5,
                started_at: Some(Timestamp::UNIX_EPOCH),
                last_scanned_at: None,
                error: None,
            })
            .await
            .unwrap();
        let jobs = MockJobStore::new();

        ScheduledScanHandler::new(libraries.clone(), jobs.clone())
            .handle(&trigger())
            .await
            .unwrap();

        assert!(jobs.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_failing_write_is_retryable() {
        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library(
            "nightly",
            WatcherStrategy::Scheduled,
            LibraryOrigin::Local,
        ));
        libraries.set_fail_save();

        let err = ScheduledScanHandler::new(libraries, MockJobStore::new())
            .handle(&trigger())
            .await
            .unwrap_err();

        assert!(matches!(err, JobError::Retryable(_)));
    }
}

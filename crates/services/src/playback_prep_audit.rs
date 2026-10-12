use std::collections::{BTreeSet, HashMap, HashSet};

use domain::catalog::VersionId;
use domain::common::PageRequest;
use domain::diagnostics::{LibraryPrep, PlaybackPrepAudit};
use domain::error::RepositoryError;
use domain::job::{JobKind, JobQuery, JobStatus};
use domain::library::LibraryId;
use domain::repository::{CatalogRepository, JobRepository, LibraryRepository};

use crate::library::PlaybackPrepJobPayload;

const JOB_PAGE: u32 = 500;

pub struct PlaybackPrepAuditImpl<C, J, L> {
    catalog: C,
    jobs: J,
    libraries: L,
}

impl<C, J, L> PlaybackPrepAuditImpl<C, J, L> {
    pub fn new(catalog: C, jobs: J, libraries: L) -> Self {
        Self { catalog, jobs, libraries }
    }
}

#[derive(Default)]
struct Tally {
    outdated: u64,
    unprepared: u64,
    prepared_with: BTreeSet<String>,
}

impl<C, J, L> PlaybackPrepAuditImpl<C, J, L>
where
    J: JobRepository + Send + Sync,
{
    async fn being_prepared(&self) -> Result<HashSet<VersionId>, RepositoryError> {
        let query =
            JobQuery { search: Some(JobKind::PlaybackPrep.slug().to_owned()), active_only: true };
        let mut versions = HashSet::new();
        let mut offset = 0;
        loop {
            let page = self.jobs.list_page(&query, PageRequest { offset, limit: JOB_PAGE }).await?;
            let fetched = page.len();
            versions.extend(
                page.into_iter()
                    .filter(|job| job.kind == JobKind::PlaybackPrep)
                    .filter(|job| matches!(job.status, JobStatus::Queued | JobStatus::Running))
                    .filter_map(|job| PlaybackPrepJobPayload::decode(&job.payload).ok())
                    .map(|payload| payload.version_id),
            );
            if fetched < JOB_PAGE as usize {
                return Ok(versions);
            }
            offset += JOB_PAGE;
        }
    }
}

impl<C, J, L> PlaybackPrepAudit for PlaybackPrepAuditImpl<C, J, L>
where
    C: CatalogRepository + Send + Sync,
    J: JobRepository + Send + Sync,
    L: LibraryRepository + Send + Sync,
{
    async fn libraries(&self, ffmpeg: Option<&str>) -> Result<Vec<LibraryPrep>, RepositoryError> {
        let pending = self.being_prepared().await?;
        let mut tallies: HashMap<LibraryId, Tally> = HashMap::new();
        for state in self.catalog.playback_prep_states().await? {
            let tally = tallies.entry(state.library).or_default();
            if pending.contains(&state.version) {
                continue;
            }
            match state.prepared_with {
                None => tally.unprepared += 1,
                Some(made) if ffmpeg.is_some_and(|running| running != made) => {
                    tally.outdated += 1;
                    tally.prepared_with.insert(made);
                }
                Some(_) => {}
            }
        }
        let mut found: Vec<LibraryPrep> = self
            .libraries
            .list()
            .await?
            .into_iter()
            .filter_map(|library| {
                let tally = tallies.remove(&library.id)?;
                (tally.outdated + tally.unprepared > 0).then(|| LibraryPrep {
                    library: library.id,
                    name: library.name,
                    outdated: tally.outdated,
                    unprepared: tally.unprepared,
                    prepared_with: tally.prepared_with.into_iter().collect(),
                })
            })
            .collect();
        found.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.library.0.cmp(&b.library.0)));
        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{MovieId, TitleId, Version};
    use domain::common::Quality;
    use domain::job::JobPriority;
    use domain::library::{Library, LibraryKind, LibraryOrigin, WatcherStrategy};
    use domain::media::PlaybackPrep;
    use jiff::Timestamp;
    use mocks::{MockCatalogRepo, MockJobStore, MockLibraryRepo};

    use crate::job::queued_job;

    type Audit = PlaybackPrepAuditImpl<MockCatalogRepo, MockJobStore, MockLibraryRepo>;

    fn library(id: &str, name: &str) -> Library {
        Library {
            id: LibraryId(id.into()),
            name: name.into(),
            origin: LibraryOrigin::Local,
            kind: LibraryKind::Movie,
            sort_articles: Vec::new(),
            roots: vec!["/media".into()],
            watcher: WatcherStrategy::Manual,
            scan_schedule: None,
            metadata_sources: Vec::new(),
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn audit() -> (Audit, MockCatalogRepo, MockJobStore) {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library("films", "Films"));
        libraries.insert_library(library("shows", "Shows"));
        libraries.insert_library(library("clean", "Clean"));
        (PlaybackPrepAuditImpl::new(catalog.clone(), jobs.clone(), libraries), catalog, jobs)
    }

    async fn version(catalog: &MockCatalogRepo, id: &str, library: &str, prepared: Option<&str>) {
        catalog
            .upsert_version(Version {
                id: VersionId(id.into()),
                title: TitleId::Movie(MovieId(format!("m-{id}"))),
                library: LibraryId(library.into()),
                quality: Quality::Hd,
                container: "mkv".into(),
                path: format!("/media/{id}.mkv"),
                size_bytes: 1,
                duration_ms: 1_000,
                available: true,
                added_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .unwrap();
        if let Some(prepared_with) = prepared {
            let prep = PlaybackPrep {
                keyframes_us: vec![0],
                extracted_subtitles: Vec::new(),
                prepared_with: prepared_with.into(),
            };
            catalog.set_playback_prep(&VersionId(id.into()), &prep).await.unwrap();
        }
    }

    async fn preparing(jobs: &MockJobStore, id: &str, status: JobStatus) {
        let payload = PlaybackPrepJobPayload { version_id: VersionId(id.into()) }.encode();
        let mut job =
            queued_job(JobKind::PlaybackPrep, JobPriority::High, payload, None, Timestamp::now());
        job.status = status;
        jobs.enqueue(job).await.unwrap();
    }

    #[tokio::test]
    async fn each_library_lists_what_another_ffmpeg_prepared_and_what_is_not_prepared() {
        let (audit, catalog, _jobs) = audit();
        version(&catalog, "f1", "films", Some("5.1.9-0+deb12u1")).await;
        version(&catalog, "f2", "films", Some("5.1.8")).await;
        version(&catalog, "f3", "films", Some("9.0.2")).await;
        version(&catalog, "f4", "films", None).await;
        version(&catalog, "s1", "shows", None).await;
        version(&catalog, "c1", "clean", Some("9.0.2")).await;

        let found = audit.libraries(Some("9.0.2")).await.unwrap();

        assert_eq!(
            found,
            vec![
                LibraryPrep {
                    library: LibraryId("films".into()),
                    name: "Films".into(),
                    outdated: 2,
                    unprepared: 1,
                    prepared_with: vec!["5.1.8".into(), "5.1.9-0+deb12u1".into()],
                },
                LibraryPrep {
                    library: LibraryId("shows".into()),
                    name: "Shows".into(),
                    outdated: 0,
                    unprepared: 1,
                    prepared_with: Vec::new(),
                },
            ]
        );
    }

    #[tokio::test]
    async fn without_a_known_ffmpeg_only_unprepared_versions_count() {
        let (audit, catalog, _jobs) = audit();
        version(&catalog, "f1", "films", Some("5.1.9-0+deb12u1")).await;
        version(&catalog, "s1", "shows", None).await;

        let found = audit.libraries(None).await.unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].library, LibraryId("shows".into()));
    }

    #[tokio::test]
    async fn a_version_whose_preparation_is_queued_or_running_is_not_reported() {
        let (audit, catalog, jobs) = audit();
        version(&catalog, "f1", "films", None).await;
        version(&catalog, "f2", "films", None).await;
        version(&catalog, "f3", "films", None).await;
        preparing(&jobs, "f1", JobStatus::Queued).await;
        preparing(&jobs, "f2", JobStatus::Running).await;
        preparing(&jobs, "f3", JobStatus::Failed).await;
        jobs.enqueue(queued_job(
            JobKind::Trickplay,
            JobPriority::Normal,
            "{}".into(),
            None,
            Timestamp::now(),
        ))
        .await
        .unwrap();

        let found = audit.libraries(Some("9.0.2")).await.unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].unprepared, 1, "only the version whose job failed is still owed");
    }

    #[tokio::test]
    async fn a_version_another_ffmpeg_prepared_is_not_reported_once_it_is_read_again() {
        let (audit, catalog, jobs) = audit();
        version(&catalog, "f1", "films", Some("5.1.9")).await;
        version(&catalog, "f2", "films", Some("5.1.9")).await;
        preparing(&jobs, "f1", JobStatus::Queued).await;

        let found = audit.libraries(Some("9.0.2")).await.unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].outdated, 1, "the re-read already queued is not asked for again");
    }

    #[tokio::test]
    async fn every_page_of_queued_preparations_is_read() {
        let (audit, catalog, jobs) = audit();
        for index in 0..=JOB_PAGE {
            let id = format!("f{index}");
            version(&catalog, &id, "films", None).await;
            preparing(&jobs, &id, JobStatus::Queued).await;
        }

        assert!(audit.libraries(Some("9.0.2")).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_store_that_cannot_be_read_is_an_error() {
        let (audit, catalog, _jobs) = audit();
        catalog.set_fail();
        assert!(audit.libraries(Some("9.0.2")).await.is_err());
    }
}

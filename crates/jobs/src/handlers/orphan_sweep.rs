use std::collections::HashSet;
use std::future::Future;

use domain::error::RepositoryError;
use domain::job::Job;
use domain::media::{DerivedAssetDir, DerivedAssetStore};
use domain::repository::CatalogRepository;
use jiff::{SignedDuration, Timestamp};

use crate::error::JobError;
use crate::job_handler::JobHandler;

const OWNER_CHUNK: usize = 500;

pub struct OrphanSweepHandler<C, A, S, T, E> {
    catalog: C,
    artwork: A,
    subtitles: S,
    trickplay: T,
    extraction: E,
    grace: SignedDuration,
}

impl<C, A, S, T, E> OrphanSweepHandler<C, A, S, T, E> {
    pub fn new(
        catalog: C,
        artwork: A,
        subtitles: S,
        trickplay: T,
        extraction: E,
        grace: SignedDuration,
    ) -> Self {
        Self { catalog, artwork, subtitles, trickplay, extraction, grace }
    }
}

impl<C, A, S, T, E> OrphanSweepHandler<C, A, S, T, E>
where
    C: CatalogRepository + Send + Sync,
    A: DerivedAssetStore,
    S: DerivedAssetStore,
    T: DerivedAssetStore,
    E: DerivedAssetStore,
{
    async fn sweep<F, G>(
        &self,
        store: &impl DerivedAssetStore,
        live_of: impl Fn(Vec<String>) -> F,
        live_paths_of: impl Fn(Vec<String>) -> G,
        cutoff: Timestamp,
    ) -> Result<u64, JobError>
    where
        F: Future<Output = Result<HashSet<String>, RepositoryError>>,
        G: Future<Output = Result<HashSet<String>, RepositoryError>>,
    {
        let (mut removed, alive) = self.sweep_dirs(store, live_of, cutoff).await?;
        removed += self.sweep_files(store, &alive, live_paths_of, cutoff).await?;
        tracing::info!(store = store.label(), removed, "orphan sweep complete");
        Ok(removed)
    }

    async fn sweep_dirs<F>(
        &self,
        store: &impl DerivedAssetStore,
        live_of: impl Fn(Vec<String>) -> F,
        cutoff: Timestamp,
    ) -> Result<(u64, Vec<String>), JobError>
    where
        F: Future<Output = Result<HashSet<String>, RepositoryError>>,
    {
        let dirs = store.list_dirs().await.map_err(|err| JobError::Retryable(err.to_string()))?;
        let stale: Vec<&DerivedAssetDir> =
            dirs.iter().filter(|dir| dir.modified_at < cutoff).collect();
        let mut live = HashSet::new();
        for chunk in stale.chunks(OWNER_CHUNK) {
            let owners = chunk.iter().map(|dir| dir.owner.clone()).collect();
            live.extend(live_of(owners).await.map_err(|err| JobError::Retryable(err.to_string()))?);
        }
        let mut removed = 0;
        let mut alive = Vec::new();
        for dir in stale {
            if live.contains(&dir.owner) {
                alive.push(dir.owner.clone());
                continue;
            }
            match store.remove_dir(&dir.owner).await {
                Ok(()) => removed += 1,
                Err(err) => {
                    #[rustfmt::skip]
                    tracing::warn!(store = store.label(), owner = %dir.owner, "removing an orphaned directory failed: {err}");
                }
            }
        }
        Ok((removed, alive))
    }

    async fn sweep_files<G>(
        &self,
        store: &impl DerivedAssetStore,
        owners: &[String],
        live_paths_of: impl Fn(Vec<String>) -> G,
        cutoff: Timestamp,
    ) -> Result<u64, JobError>
    where
        G: Future<Output = Result<HashSet<String>, RepositoryError>>,
    {
        let mut live = HashSet::new();
        for chunk in owners.chunks(OWNER_CHUNK) {
            live.extend(
                live_paths_of(chunk.to_vec())
                    .await
                    .map_err(|err| JobError::Retryable(err.to_string()))?,
            );
        }
        let mut removed = 0;
        for owner in owners {
            let files = store
                .list_files(owner)
                .await
                .map_err(|err| JobError::Retryable(err.to_string()))?;
            for file in files {
                if file.modified_at >= cutoff || live.contains(&file.path) {
                    continue;
                }
                match store.remove_file(&file.path).await {
                    Ok(()) => removed += 1,
                    Err(err) => {
                        #[rustfmt::skip]
                        tracing::warn!(store = store.label(), path = %file.path, "removing a superseded file failed: {err}");
                    }
                }
            }
        }
        Ok(removed)
    }
}

impl<C, A, S, T, E> JobHandler for OrphanSweepHandler<C, A, S, T, E>
where
    C: CatalogRepository + Send + Sync,
    A: DerivedAssetStore,
    S: DerivedAssetStore,
    T: DerivedAssetStore,
    E: DerivedAssetStore,
{
    async fn handle(&self, _job: &Job) -> Result<(), JobError> {
        let Ok(cutoff) = Timestamp::now().checked_sub(self.grace) else {
            return Err(JobError::Permanent("sweep grace period is out of range".into()));
        };
        let artwork =
            |owners: Vec<String>| async move { self.catalog.live_artwork_ids(&owners).await };
        let versions =
            |owners: Vec<String>| async move { self.catalog.live_version_ids(&owners).await };
        let artwork_paths =
            |owners: Vec<String>| async move { self.catalog.live_artwork_paths(&owners).await };
        let subtitle_paths =
            |owners: Vec<String>| async move { self.catalog.live_subtitle_paths(&owners).await };
        let trickplay_paths =
            |owners: Vec<String>| async move { self.catalog.live_trickplay_paths(&owners).await };
        self.sweep(&self.artwork, artwork, artwork_paths, cutoff).await?;
        self.sweep(&self.subtitles, versions, subtitle_paths, cutoff).await?;
        self.sweep(&self.trickplay, versions, trickplay_paths, cutoff).await?;
        let (removed, _) = self.sweep_dirs(&self.extraction, versions, cutoff).await?;
        #[rustfmt::skip]
        tracing::info!(store = self.extraction.label(), removed, "orphan sweep complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use domain::catalog::{
        ArtworkId, ArtworkOwner, ArtworkRef, ArtworkWidth, MovieId, TitleId, Version, VersionId,
    };
    use domain::common::Quality;
    use domain::error::CacheError;
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use domain::library::LibraryId;
    use domain::media::{
        DerivedAssetDir, DerivedAssetFile, SubtitleFile, SubtitleFileId, SubtitleFormat,
        SubtitleSource, TrickplayAsset,
    };
    use domain::metadata::ArtworkKind;
    use mocks::MockCatalogRepo;

    use super::*;

    #[derive(Clone, Default)]
    struct StubStore {
        dirs: Arc<Mutex<Vec<DerivedAssetDir>>>,
        files: Arc<Mutex<HashMap<String, Vec<DerivedAssetFile>>>>,
        removed: Arc<Mutex<Vec<String>>>,
        removed_files: Arc<Mutex<Vec<String>>>,
        fail_removal: bool,
        fail_listing: bool,
    }

    impl StubStore {
        fn with(dirs: Vec<DerivedAssetDir>) -> Self {
            Self { dirs: Arc::new(Mutex::new(dirs)), ..Self::default() }
        }

        fn holding(dirs: Vec<DerivedAssetDir>, files: Vec<(&str, Vec<DerivedAssetFile>)>) -> Self {
            let store = Self::with(dirs);
            let mut held = store.files.lock().unwrap();
            for (owner, listing) in files {
                held.insert(owner.to_owned(), listing);
            }
            drop(held);
            store
        }

        fn removed_files(&self) -> Vec<String> {
            self.removed_files.lock().unwrap().clone()
        }

        fn failing(dirs: Vec<DerivedAssetDir>) -> Self {
            Self { fail_removal: true, ..Self::with(dirs) }
        }

        fn unreadable() -> Self {
            Self { fail_listing: true, ..Self::with(Vec::new()) }
        }

        fn removed(&self) -> Vec<String> {
            self.removed.lock().unwrap().clone()
        }
    }

    impl DerivedAssetStore for StubStore {
        fn label(&self) -> &'static str {
            "stub"
        }

        async fn list_dirs(&self) -> Result<Vec<DerivedAssetDir>, CacheError> {
            if self.fail_listing {
                return Err(CacheError::Io("the disk went away".into()));
            }
            Ok(self.dirs.lock().unwrap().clone())
        }

        async fn remove_dir(&self, owner: &str) -> Result<(), CacheError> {
            if self.fail_removal {
                return Err(CacheError::Io("boom".into()));
            }
            self.removed.lock().unwrap().push(owner.to_owned());
            Ok(())
        }

        async fn list_files(&self, owner: &str) -> Result<Vec<DerivedAssetFile>, CacheError> {
            if self.fail_listing {
                return Err(CacheError::Io("the disk went away".into()));
            }
            Ok(self.files.lock().unwrap().get(owner).cloned().unwrap_or_default())
        }

        async fn remove_file(&self, path: &str) -> Result<(), CacheError> {
            if self.fail_removal {
                return Err(CacheError::Io("boom".into()));
            }
            self.removed_files.lock().unwrap().push(path.to_owned());
            Ok(())
        }
    }

    fn file(path: &str, age: SignedDuration) -> DerivedAssetFile {
        DerivedAssetFile {
            path: path.into(),
            modified_at: Timestamp::now().checked_sub(age).unwrap(),
        }
    }

    fn dir(owner: &str, age: SignedDuration) -> DerivedAssetDir {
        DerivedAssetDir {
            owner: owner.into(),
            modified_at: Timestamp::now().checked_sub(age).unwrap(),
        }
    }

    fn version(id: &str) -> Version {
        Version {
            id: VersionId(id.into()),
            title: TitleId::Movie(MovieId("m1".into())),
            library: LibraryId("lib".into()),
            quality: Quality::Hd,
            container: "mkv".into(),
            path: format!("/m/{id}.mkv"),
            size_bytes: 1,
            duration_ms: 1000,
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn sweep_job() -> Job {
        let now = Timestamp::UNIX_EPOCH;
        Job {
            id: JobId("sweep".into()),
            kind: JobKind::OrphanSweep,
            status: JobStatus::Running,
            priority: JobPriority::Normal,
            payload: String::new(),
            attempts: 0,
            progress: 0.0,
            available_at: now,
            last_error: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            finished_at: None,
            parent_id: None,
        }
    }

    async fn catalog_with_artwork(id: &str) -> MockCatalogRepo {
        let catalog = MockCatalogRepo::new();
        catalog
            .set_artwork(
                &ArtworkOwner::Movie(MovieId("m1".into())),
                &[ArtworkRef {
                    id: ArtworkId(id.into()),
                    kind: ArtworkKind::Poster,
                    widths: vec![ArtworkWidth::new(480, format!("/art/{id}/480.jpg"))],
                }],
            )
            .await
            .unwrap();
        catalog
    }

    async fn run(artwork: StubStore, grace: SignedDuration) -> StubStore {
        OrphanSweepHandler::new(
            catalog_with_artwork("live").await,
            artwork.clone(),
            StubStore::default(),
            StubStore::default(),
            StubStore::default(),
            grace,
        )
        .handle(&sweep_job())
        .await
        .unwrap();
        artwork
    }

    #[tokio::test]
    async fn an_unowned_directory_past_the_grace_period_is_removed() {
        let store = StubStore::with(vec![dir("gone", SignedDuration::from_hours(48))]);
        let store = run(store, SignedDuration::from_hours(24)).await;
        assert_eq!(store.removed(), vec!["gone".to_owned()]);
    }

    #[tokio::test]
    async fn a_directory_with_a_live_owner_is_kept() {
        let store = StubStore::with(vec![dir("live", SignedDuration::from_hours(48))]);
        let store = run(store, SignedDuration::from_hours(24)).await;
        assert!(store.removed().is_empty());
    }

    #[tokio::test]
    async fn a_recent_directory_is_kept_even_without_an_owner() {
        let store = StubStore::with(vec![dir("fresh", SignedDuration::from_secs(300))]);
        let store = run(store, SignedDuration::from_hours(24)).await;
        assert!(store.removed().is_empty());
    }

    #[tokio::test]
    async fn a_superseded_file_inside_a_live_directory_is_removed() {
        let store = StubStore::holding(
            vec![dir("live", SignedDuration::from_hours(48))],
            vec![(
                "live",
                vec![
                    file("/art/live/480.jpg", SignedDuration::from_hours(48)),
                    file("/art/live/960.jpg", SignedDuration::from_hours(48)),
                ],
            )],
        );
        let store = run(store, SignedDuration::from_hours(24)).await;

        assert!(store.removed().is_empty(), "the owner is still live");
        assert_eq!(
            store.removed_files(),
            vec!["/art/live/960.jpg".to_owned()],
            "only the width the catalog no longer references goes"
        );
    }

    #[tokio::test]
    async fn a_file_written_inside_the_grace_window_is_never_removed() {
        let store = StubStore::holding(
            vec![dir("live", SignedDuration::from_hours(48))],
            vec![("live", vec![file("/art/live/1440.jpg", SignedDuration::from_secs(30))])],
        );
        let store = run(store, SignedDuration::from_hours(24)).await;

        assert!(
            store.removed_files().is_empty(),
            "a running job writes the file before the catalog row exists, so the grace \
             period has to be measured per file, not per directory"
        );
    }

    #[tokio::test]
    async fn a_dead_owner_still_loses_its_whole_directory_without_a_per_file_pass() {
        let store = StubStore::holding(
            vec![dir("gone", SignedDuration::from_hours(48))],
            vec![("gone", vec![file("/art/gone/480.jpg", SignedDuration::from_hours(48))])],
        );
        let store = run(store, SignedDuration::from_hours(24)).await;

        assert_eq!(store.removed(), vec!["gone".to_owned()]);
        assert!(store.removed_files().is_empty());
    }

    #[tokio::test]
    async fn a_failed_file_removal_does_not_abort_the_sweep() {
        let store = StubStore {
            fail_removal: true,
            ..StubStore::holding(
                vec![dir("live", SignedDuration::from_hours(48))],
                vec![("live", vec![file("/art/live/960.jpg", SignedDuration::from_hours(48))])],
            )
        };
        let store = run(store, SignedDuration::from_hours(24)).await;
        assert!(store.removed_files().is_empty());
    }

    #[tokio::test]
    async fn a_stale_subtitle_beside_a_live_one_is_the_case_that_started_this() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version("v1"));
        catalog
            .set_subtitle_files(
                &VersionId("v1".into()),
                &[SubtitleFile {
                    id: SubtitleFileId("opensubtitles:v1:42".into()),
                    version: VersionId("v1".into()),
                    language: None,
                    format: SubtitleFormat::Srt,
                    source: SubtitleSource::OpenSubtitles,
                    path: "/subs/v1/42.srt".into(),
                    translated_from: None,
                    label: None,
                    pinned: false,
                }],
            )
            .await
            .unwrap();
        let subtitles = StubStore::holding(
            vec![dir("v1", SignedDuration::from_hours(48))],
            vec![(
                "v1",
                vec![
                    file("/subs/v1/42.srt", SignedDuration::from_hours(48)),
                    file("/subs/v1/99.srt", SignedDuration::from_hours(48)),
                ],
            )],
        );

        OrphanSweepHandler::new(
            catalog,
            StubStore::default(),
            subtitles.clone(),
            StubStore::default(),
            StubStore::default(),
            SignedDuration::from_hours(24),
        )
        .handle(&sweep_job())
        .await
        .unwrap();

        assert!(subtitles.removed().is_empty());
        assert_eq!(subtitles.removed_files(), vec!["/subs/v1/99.srt".to_owned()]);
    }

    #[tokio::test]
    async fn a_trickplay_sheet_the_catalog_dropped_is_collected() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version("v1"));
        catalog
            .set_trickplay(
                &VersionId("v1".into()),
                &[TrickplayAsset {
                    version: VersionId("v1".into()),
                    interval_ms: 5_000,
                    columns: 8,
                    rows: 8,
                    tile_width: 320,
                    tile_height: 180,
                    sheet_paths: vec!["/tp/v1/sheet-001.jpg".into()],
                }],
            )
            .await
            .unwrap();
        let trickplay = StubStore::holding(
            vec![dir("v1", SignedDuration::from_hours(48))],
            vec![(
                "v1",
                vec![
                    file("/tp/v1/sheet-001.jpg", SignedDuration::from_hours(48)),
                    file("/tp/v1/sheet-002.jpg", SignedDuration::from_hours(48)),
                ],
            )],
        );

        OrphanSweepHandler::new(
            catalog,
            StubStore::default(),
            StubStore::default(),
            trickplay.clone(),
            StubStore::default(),
            SignedDuration::from_hours(24),
        )
        .handle(&sweep_job())
        .await
        .unwrap();

        assert!(trickplay.removed().is_empty());
        assert_eq!(
            trickplay.removed_files(),
            vec!["/tp/v1/sheet-002.jpg".to_owned()],
            "a regenerate with fewer sheets leaves the tail behind without this"
        );
    }

    #[tokio::test]
    async fn a_failed_removal_does_not_abort_the_sweep() {
        let store = StubStore::failing(vec![
            dir("one", SignedDuration::from_hours(48)),
            dir("two", SignedDuration::from_hours(48)),
        ]);
        OrphanSweepHandler::new(
            catalog_with_artwork("live").await,
            store,
            StubStore::default(),
            StubStore::default(),
            StubStore::default(),
            SignedDuration::from_hours(24),
        )
        .handle(&sweep_job())
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn versions_guard_the_subtitle_and_trickplay_stores() {
        let subtitles = StubStore::with(vec![dir("v-gone", SignedDuration::from_hours(48))]);
        let trickplay = StubStore::with(vec![dir("v-gone", SignedDuration::from_hours(48))]);
        OrphanSweepHandler::new(
            MockCatalogRepo::new(),
            StubStore::default(),
            subtitles.clone(),
            trickplay.clone(),
            StubStore::default(),
            SignedDuration::from_hours(24),
        )
        .handle(&sweep_job())
        .await
        .unwrap();
        assert_eq!(subtitles.removed(), vec!["v-gone".to_owned()]);
        assert_eq!(trickplay.removed(), vec!["v-gone".to_owned()]);
        assert_eq!(subtitles.label(), "stub");
    }

    #[tokio::test]
    async fn a_dead_version_loses_its_extracted_subtitles() {
        let extraction = StubStore::with(vec![dir("v-gone", SignedDuration::from_hours(48))]);
        OrphanSweepHandler::new(
            MockCatalogRepo::new(),
            StubStore::default(),
            StubStore::default(),
            StubStore::default(),
            extraction.clone(),
            SignedDuration::from_hours(24),
        )
        .handle(&sweep_job())
        .await
        .unwrap();
        assert_eq!(extraction.removed(), vec!["v-gone".to_owned()]);
    }

    #[tokio::test]
    async fn a_live_version_keeps_extracts_no_database_row_knows_about() {
        let catalog = MockCatalogRepo::new();
        catalog.add_version(version("v1"));
        let extraction = StubStore::holding(
            vec![dir("v1", SignedDuration::from_hours(48))],
            vec![(
                "v1",
                vec![file("/extract/v1/embedded-3-abc.vtt", SignedDuration::from_hours(48))],
            )],
        );
        OrphanSweepHandler::new(
            catalog,
            StubStore::default(),
            StubStore::default(),
            StubStore::default(),
            extraction.clone(),
            SignedDuration::from_hours(24),
        )
        .handle(&sweep_job())
        .await
        .unwrap();
        assert!(extraction.removed().is_empty());
        #[rustfmt::skip]
        assert!(extraction.removed_files().is_empty(), "an extract is in no subtitle row, so a file pass would delete every one of them");
    }

    #[tokio::test]
    async fn a_grace_period_that_cannot_be_subtracted_is_permanent() {
        let grace = SignedDuration::MAX;
        let error = OrphanSweepHandler::new(
            MockCatalogRepo::new(),
            StubStore::default(),
            StubStore::default(),
            StubStore::default(),
            StubStore::default(),
            grace,
        )
        .handle(&sweep_job())
        .await
        .unwrap_err();
        assert!(matches!(error, JobError::Permanent(_)));
    }

    #[tokio::test]
    async fn a_store_that_cannot_be_listed_makes_the_sweep_retryable() {
        let error = OrphanSweepHandler::new(
            MockCatalogRepo::new(),
            StubStore::unreadable(),
            StubStore::default(),
            StubStore::default(),
            StubStore::default(),
            SignedDuration::from_hours(24),
        )
        .handle(&sweep_job())
        .await
        .unwrap_err();
        assert!(matches!(error, JobError::Retryable(_)));
    }
}

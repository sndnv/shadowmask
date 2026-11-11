use domain::error::{FetchError, RepositoryError};
use domain::job::Job;
use domain::library::{DiscoveredFile, LibraryOrigin};
use domain::media::{FetchSpec, MediaFetcher, MediaProbe};
use domain::repository::LibraryRepository;
use services::library::{FetchJobPayload, ResolveIngester};

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct FetchJobHandler<R, F, P, E> {
    repo: R,
    fetcher: F,
    probe: P,
    ingester: E,
}

impl<R, F, P, E> FetchJobHandler<R, F, P, E> {
    pub fn new(repo: R, fetcher: F, probe: P, ingester: E) -> Self {
        Self { repo, fetcher, probe, ingester }
    }
}

impl<R, F, P, E> JobHandler for FetchJobHandler<R, F, P, E>
where
    R: LibraryRepository + Send + Sync,
    F: MediaFetcher + Send + Sync,
    P: MediaProbe + Send + Sync,
    E: ResolveIngester + Send + Sync,
{
    async fn handle(&self, job: &Job) -> Result<(), JobError> {
        let payload = FetchJobPayload::decode(&job.payload)
            .map_err(|e| JobError::Permanent(format!("invalid fetch payload: {e}")))?;
        let library =
            self.repo.get(&payload.library).await.map_err(retryable)?.ok_or_else(|| {
                JobError::Permanent(format!("library not found: {}", payload.library.0))
            })?;
        if library.origin != LibraryOrigin::External {
            return Err(JobError::Permanent(format!(
                "library [{}] is not an external fetch library",
                library.id.0
            )));
        }
        let root = library.roots.first().ok_or_else(|| {
            JobError::Permanent(format!("external library [{}] has no root", library.id.0))
        })?;
        let dest_dir = payload.destination_dir(library.kind, root, &job.id.0);
        let stem = payload.filename_stem(library.kind);
        tracing::info!("fetching [{}] into [{}]", payload.source_url, dest_dir);
        let fetched = self
            .fetcher
            .fetch(&FetchSpec {
                url: payload.source_url.clone(),
                dest_dir,
                filename_stem: stem,
                options: payload.options.clone(),
            })
            .await
            .map_err(fetch_error)?;
        let probe = self
            .probe
            .probe(&fetched.path)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?;
        let file = DiscoveredFile {
            library: payload.library.clone(),
            path: fetched.path,
            size_bytes: fetched.size_bytes,
            subtitle_siblings: Vec::new(),
            probe,
        };
        let parsed = payload.parsed(library.kind, &file.path);
        self.ingester
            .ingest_fetched(&library, &file, &parsed, &fetched.site, Some(&job.id))
            .await
            .map_err(retryable)?;
        Ok(())
    }
}

fn retryable(err: RepositoryError) -> JobError {
    JobError::Retryable(err.to_string())
}

fn fetch_error(err: FetchError) -> JobError {
    match err {
        FetchError::Spawn(m) | FetchError::NoOutput(m) => JobError::Permanent(m),
        FetchError::Download(m) | FetchError::Io(m) => JobError::Retryable(m),
        refused @ (FetchError::Unsupported | FetchError::InvalidFormat(_)) => {
            JobError::Permanent(refused.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::common::PageRequest;
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use domain::library::{Library, LibraryId, LibraryKind, LibraryOrigin, WatcherStrategy};
    use domain::media::{FetchOptions, FetchedMedia, SiteDetails};
    use domain::repository::{CatalogRepository, JobRepository};
    use jiff::Timestamp;
    use mocks::{
        MockCatalogRepo, MockJobStore, MockLibraryRepo, MockMediaProbe, MockMetadataProvider,
    };
    use services::library::Enricher;
    use std::sync::Mutex;

    enum Outcome {
        Ok,
        Permanent,
        Retryable,
        Unsupported,
        InvalidFormat,
    }

    struct MockFetcher {
        outcome: Outcome,
        seen: Mutex<Vec<FetchSpec>>,
    }

    impl MediaFetcher for MockFetcher {
        async fn fetch(&self, request: &FetchSpec) -> Result<FetchedMedia, FetchError> {
            self.seen.lock().unwrap().push(request.clone());
            match self.outcome {
                Outcome::Ok => Ok(FetchedMedia {
                    path: format!("{}/{}.mkv", request.dest_dir, request.filename_stem),
                    size_bytes: 4096,
                    site: SiteDetails {
                        upload_date: Some("20260801".into()),
                        ..SiteDetails::default()
                    },
                }),
                Outcome::Permanent => Err(FetchError::NoOutput("no output".into())),
                Outcome::Retryable => Err(FetchError::Download("network".into())),
                Outcome::Unsupported => Err(FetchError::Unsupported),
                Outcome::InvalidFormat => {
                    Err(FetchError::InvalidFormat("Invalid format specification: x".into()))
                }
            }
        }
    }

    fn external_library() -> Library {
        Library {
            id: LibraryId("ext".into()),
            name: "Ext".into(),
            origin: LibraryOrigin::External,
            kind: LibraryKind::Movie,
            roots: vec!["/ext".into()],
            sort_articles: Vec::new(),
            watcher: WatcherStrategy::Manual,
            scan_schedule: None,
            metadata_sources: Vec::new(),
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn local_library() -> Library {
        Library { origin: LibraryOrigin::Local, ..external_library() }
    }

    fn page() -> PageRequest {
        PageRequest { offset: 0, limit: 10 }
    }

    fn payload() -> FetchJobPayload {
        FetchJobPayload {
            library: LibraryId("ext".into()),
            source_url: "https://example.com/watch?v=abc".into(),
            title: "The Matrix".into(),
            year: Some(1999),
            external_id: None,
            season: None,
            episode: None,
            episode_title: None,
            options: FetchOptions::default(),
        }
    }

    fn job(payload: String) -> Job {
        let now = Timestamp::UNIX_EPOCH;
        Job {
            id: JobId("job-1".into()),
            kind: JobKind::Fetch,
            status: JobStatus::Running,
            priority: JobPriority::Normal,
            payload,
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

    #[allow(clippy::type_complexity)]
    fn handler(
        repo: MockLibraryRepo,
        outcome: Outcome,
        probe: MockMediaProbe,
    ) -> (
        FetchJobHandler<
            MockLibraryRepo,
            MockFetcher,
            MockMediaProbe,
            Enricher<MockCatalogRepo, MockMetadataProvider, MockJobStore, MockLibraryRepo>,
        >,
        MockCatalogRepo,
        MockJobStore,
    ) {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let ingester = Enricher::new(
            catalog.clone(),
            None::<MockMetadataProvider>,
            jobs.clone(),
            MockLibraryRepo::new(),
        );
        let fetcher = MockFetcher { outcome, seen: Mutex::new(Vec::new()) };
        (FetchJobHandler::new(repo, fetcher, probe, ingester), catalog, jobs)
    }

    #[tokio::test]
    async fn the_download_options_reach_the_fetcher() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(external_library());
        let (h, _, _) = handler(repo, Outcome::Ok, MockMediaProbe::new());
        let options = FetchOptions {
            skip_note: Some("Premium".into()),
            site_subtitles: true,
            ..FetchOptions::default()
        };

        let queued = FetchJobPayload { options: options.clone(), ..payload() };
        h.handle(&job(queued.encode())).await.unwrap();

        let seen = h.fetcher.seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].options, options);
    }

    #[tokio::test]
    async fn an_unsupported_site_is_permanent_and_points_at_the_stream_link() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(external_library());
        let (h, _, _) = handler(repo, Outcome::Unsupported, MockMediaProbe::new());

        let error = h.handle(&job(payload().encode())).await.unwrap_err();

        let JobError::Permanent(message) = error else { panic!("not a permanent failure") };
        assert!(message.contains("direct link to its stream (.m3u8, .mpd"), "{message}");
    }

    #[tokio::test]
    async fn a_format_yt_dlp_refuses_is_permanent_and_names_the_reason() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(external_library());
        let (h, _, _) = handler(repo, Outcome::InvalidFormat, MockMediaProbe::new());

        let error = h.handle(&job(payload().encode())).await.unwrap_err();

        let JobError::Permanent(message) = error else { panic!("not a permanent failure") };
        assert_eq!(message, "the format is not valid: [Invalid format specification: x]");
    }

    #[tokio::test]
    async fn fetches_ingests_and_parents_the_fanout() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(external_library());
        let (h, catalog, jobs) = handler(repo, Outcome::Ok, MockMediaProbe::new());

        h.handle(&job(payload().encode())).await.unwrap();

        assert_eq!(
            catalog.list_library_versions(&LibraryId("ext".into()), page()).await.unwrap().total,
            1
        );
        let enqueued = jobs.list().await.unwrap();
        assert!(!enqueued.is_empty());
        assert!(enqueued.iter().all(|j| j.parent_id == Some(JobId("job-1".into()))));
    }

    #[tokio::test]
    async fn the_title_the_admin_typed_survives_reading_as_a_release_name() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(external_library());
        let (h, catalog, _) = handler(repo, Outcome::Ok, MockMediaProbe::new());

        let mut payload = payload();
        payload.title = "OPEN STUDIO 4K DEMO REEL 2160P 60FPS".into();
        payload.year = None;
        h.handle(&job(payload.encode())).await.unwrap();

        let movies = catalog.list_movies(page()).await.unwrap();
        assert_eq!(
            movies.items[0].title, "OPEN STUDIO 4K DEMO REEL 2160P 60FPS",
            "the fetch knows the title, so it must not be re-derived from the \
             filename it just wrote"
        );
    }

    #[tokio::test]
    async fn a_movie_is_written_into_its_own_title_directory() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(external_library());
        let (h, catalog, _) = handler(repo, Outcome::Ok, MockMediaProbe::new());

        let mut payload = payload();
        payload.external_id = Some("603".into());
        h.handle(&job(payload.encode())).await.unwrap();

        let versions =
            catalog.list_library_versions(&LibraryId("ext".into()), page()).await.unwrap();
        assert_eq!(
            versions.items[0].path,
            "/ext/The Matrix (1999)/The Matrix (1999) [tmdbid-603].mkv"
        );
    }

    #[tokio::test]
    async fn an_episode_is_written_under_a_padded_season_directory() {
        let repo = MockLibraryRepo::new();
        let mut library = external_library();
        library.kind = LibraryKind::Tv;
        repo.insert_library(library);
        let (h, catalog, _) = handler(repo, Outcome::Ok, MockMediaProbe::new());

        let mut payload = payload();
        payload.title = "Great Show".into();
        payload.year = None;
        payload.season = Some(1);
        payload.episode = Some(2);
        payload.episode_title = Some("Pilot".into());
        h.handle(&job(payload.encode())).await.unwrap();

        let versions =
            catalog.list_library_versions(&LibraryId("ext".into()), page()).await.unwrap();
        assert_eq!(
            versions.items[0].path,
            "/ext/Great Show/Season 01/Great Show - S01E02 - Pilot.mkv"
        );
        assert_eq!(catalog.list_movies(page()).await.unwrap().total, 0);
        let series = catalog.list_series(page()).await.unwrap();
        let season = catalog.list_seasons(&series.items[0].id).await.unwrap()[0].clone();
        let episode = catalog.list_episodes(&season.id).await.unwrap()[0].clone();
        assert_eq!(episode.title, "Pilot");
        assert_eq!(
            episode.air_date,
            Some("2026-08-01T00:00:00Z".parse::<Timestamp>().unwrap()),
            "what the site reported reaches the episode"
        );
    }

    #[tokio::test]
    async fn a_movie_library_files_a_payload_with_episode_numbers_as_a_movie() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(external_library());
        let (h, catalog, _) = handler(repo, Outcome::Ok, MockMediaProbe::new());

        let mut payload = payload();
        payload.season = Some(1);
        payload.episode = Some(2);
        h.handle(&job(payload.encode())).await.unwrap();

        let versions =
            catalog.list_library_versions(&LibraryId("ext".into()), page()).await.unwrap();
        assert_eq!(versions.items[0].path, "/ext/The Matrix (1999)/The Matrix (1999).mkv");
        assert_eq!(catalog.list_movies(page()).await.unwrap().total, 1);
    }

    #[tokio::test]
    async fn fetch_with_external_id_ingests_via_provider_lookup() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(external_library());
        let (h, catalog, _) = handler(repo, Outcome::Ok, MockMediaProbe::new());

        let mut payload = payload();
        payload.external_id = Some("tt0133093".into());
        h.handle(&job(payload.encode())).await.unwrap();

        assert_eq!(
            catalog.list_library_versions(&LibraryId("ext".into()), page()).await.unwrap().total,
            1
        );
    }

    #[tokio::test]
    async fn invalid_payload_is_permanent() {
        let (h, _, _) = handler(MockLibraryRepo::new(), Outcome::Ok, MockMediaProbe::new());
        assert!(matches!(
            h.handle(&job("garbage".into())).await.unwrap_err(),
            JobError::Permanent(_)
        ));
    }

    #[tokio::test]
    async fn missing_library_is_permanent() {
        let (h, _, _) = handler(MockLibraryRepo::new(), Outcome::Ok, MockMediaProbe::new());
        assert!(matches!(
            h.handle(&job(payload().encode())).await.unwrap_err(),
            JobError::Permanent(_)
        ));
    }

    #[tokio::test]
    async fn non_external_library_is_permanent() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(local_library());
        let (h, _, _) = handler(repo, Outcome::Ok, MockMediaProbe::new());
        assert!(matches!(
            h.handle(&job(payload().encode())).await.unwrap_err(),
            JobError::Permanent(_)
        ));
    }

    #[tokio::test]
    async fn fetch_permanent_error_is_permanent() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(external_library());
        let (h, _, _) = handler(repo, Outcome::Permanent, MockMediaProbe::new());
        assert!(matches!(
            h.handle(&job(payload().encode())).await.unwrap_err(),
            JobError::Permanent(_)
        ));
    }

    #[tokio::test]
    async fn fetch_retryable_error_is_retryable() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(external_library());
        let (h, _, _) = handler(repo, Outcome::Retryable, MockMediaProbe::new());
        assert!(matches!(
            h.handle(&job(payload().encode())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn probe_failure_is_retryable() {
        let repo = MockLibraryRepo::new();
        repo.insert_library(external_library());
        let (h, _, _) = handler(
            repo,
            Outcome::Ok,
            MockMediaProbe::new().failing_on("/ext/The Matrix (1999)/The Matrix (1999).mkv"),
        );
        assert!(matches!(
            h.handle(&job(payload().encode())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }

    #[tokio::test]
    async fn library_without_root_is_permanent() {
        let repo = MockLibraryRepo::new();
        let mut lib = external_library();
        lib.roots = Vec::new();
        repo.insert_library(lib);
        let (h, _, _) = handler(repo, Outcome::Ok, MockMediaProbe::new());
        assert!(matches!(
            h.handle(&job(payload().encode())).await.unwrap_err(),
            JobError::Permanent(_)
        ));
    }

    #[tokio::test]
    async fn repo_get_error_is_retryable() {
        let repo = MockLibraryRepo::new();
        repo.set_fail_get();
        let (h, _, _) = handler(repo, Outcome::Ok, MockMediaProbe::new());
        assert!(matches!(
            h.handle(&job(payload().encode())).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }
}

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use tower::ServiceExt;

use api::{AppState, SubtitleSearchState, subtitle_search_router};
use contracts::Generator;
use domain::catalog::{
    Episode, EpisodeId, Movie, MovieId, Season, SeasonId, Series, SeriesId, TitleId, TitleRef,
    Version, VersionId,
};
use domain::common::{LanguageCode, Quality};
use domain::error::SubtitleError;
use domain::library::LibraryId;
use domain::media::{
    FetchedSubtitle, SubtitleCandidate, SubtitleFile, SubtitleFileId, SubtitleFormat,
    SubtitleProvider, SubtitleQuery, SubtitleSource, SubtitleStore,
};
use domain::metadata::{ExternalId, TitleEnrichment};
use domain::repository::CatalogRepository;
use jiff::Timestamp;
use mocks::MockCatalogRepo;
use tracing_test::traced_test;

const ADMIN: &str = "Bearer access:admin";
const USER: &str = "Bearer access:u1";

#[derive(Clone, Default)]
struct MockStore {
    files: Arc<Mutex<HashMap<String, String>>>,
    fail: bool,
}

impl MockStore {
    fn failing() -> Self {
        Self { fail: true, ..Self::default() }
    }
}

impl SubtitleStore for MockStore {
    async fn store(
        &self,
        version: &VersionId,
        file_id: &str,
        format: SubtitleFormat,
        content: &str,
    ) -> Result<String, SubtitleError> {
        if self.fail {
            return Err(SubtitleError::Store("disk full".into()));
        }
        let path = format!("/subs/{}/{file_id}.{}", version.0, format.extension());
        self.files.lock().unwrap().insert(path.clone(), content.to_owned());
        Ok(path)
    }
}

enum SearchMode {
    Ok(Vec<SubtitleCandidate>),
    NotFound,
    Backend,
}

enum DownloadMode {
    Ok(FetchedSubtitle),
    NotFound,
    Backend,
}

#[derive(Clone, Default)]
struct SeenQuery(Arc<Mutex<Option<SubtitleQuery>>>);

impl SeenQuery {
    fn take(&self) -> SubtitleQuery {
        self.0.lock().unwrap().clone().expect("provider was called")
    }
}

struct MockProvider {
    search: SearchMode,
    download: DownloadMode,
    seen: SeenQuery,
    downloads: Arc<AtomicUsize>,
}

impl Default for MockProvider {
    fn default() -> Self {
        Self {
            seen: SeenQuery::default(),
            downloads: Arc::default(),
            search: SearchMode::Ok(vec![SubtitleCandidate {
                file_id: "42".into(),
                language: Some(LanguageCode("en".into())),
                format: SubtitleFormat::Srt,
                release_name: Some("The.Matrix.1999.BluRay".into()),
                download_count: Some(9999),
                rating: Some(8.5),
            }]),
            download: DownloadMode::Ok(FetchedSubtitle {
                content: "1\n00:00:01,000 --> 00:00:02,000\nHi\n".into(),
                format: SubtitleFormat::Srt,
            }),
        }
    }
}

impl SubtitleProvider for MockProvider {
    async fn search(&self, query: &SubtitleQuery) -> Result<Vec<SubtitleCandidate>, SubtitleError> {
        *self.seen.0.lock().unwrap() = Some(query.clone());
        match &self.search {
            SearchMode::Ok(candidates) => Ok(candidates.clone()),
            SearchMode::NotFound => Err(SubtitleError::NotFound),
            SearchMode::Backend => Err(SubtitleError::Backend("boom".into())),
        }
    }

    async fn download(&self, _file_id: &str) -> Result<FetchedSubtitle, SubtitleError> {
        self.downloads.fetch_add(1, Ordering::SeqCst);
        match &self.download {
            DownloadMode::Ok(fetched) => Ok(fetched.clone()),
            DownloadMode::NotFound => Err(SubtitleError::NotFound),
            DownloadMode::Backend => Err(SubtitleError::Backend("boom".into())),
        }
    }
}

fn version() -> Version {
    Version {
        id: VersionId("v1".into()),
        title: TitleId::Movie(MovieId("m1".into())),
        library: LibraryId("lib".into()),
        quality: Quality::Hd,
        container: "mkv".into(),
        path: "/m/v1.mkv".into(),
        size_bytes: 1,
        duration_ms: 1000,
        available: true,
        added_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
    }
}

fn subtitle(id: &str, source: SubtitleSource, path: &str) -> SubtitleFile {
    SubtitleFile {
        id: SubtitleFileId(id.into()),
        version: VersionId("v1".into()),
        language: Some(LanguageCode("en".into())),
        format: SubtitleFormat::Srt,
        source,
        path: path.into(),
        translated_from: None,
        label: None,
        pinned: false,
    }
}

async fn seeded(files: &[SubtitleFile]) -> MockCatalogRepo {
    let catalog = MockCatalogRepo::new();
    catalog.add_version(version());
    catalog.set_subtitle_files(&VersionId("v1".into()), files).await.unwrap();
    catalog
}

fn imdb(value: &str) -> TitleEnrichment {
    TitleEnrichment {
        external_ids: vec![ExternalId { source: "imdb".into(), value: value.into() }],
        ..TitleEnrichment::default()
    }
}

async fn seeded_movie(external: Option<&str>) -> MockCatalogRepo {
    let catalog = seeded(&[]).await;
    catalog.add_movie(Movie {
        id: MovieId("m1".into()),
        title: "The Matrix".into(),
        sort_title: "matrix".into(),
        year: Some(1999),
        overview: None,
        runtime_minutes: None,
        content_rating: None,
        manually_edited: false,
        added_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
        artwork: Vec::new(),
    });
    if let Some(value) = external {
        catalog
            .set_title_enrichment(&TitleRef::Movie(MovieId("m1".into())), &imdb(value))
            .await
            .unwrap();
    }
    catalog
}

async fn seeded_episode() -> MockCatalogRepo {
    let catalog = MockCatalogRepo::new();
    catalog.add_version(Version { title: TitleId::Episode(EpisodeId("e1".into())), ..version() });
    catalog.add_series(Series {
        id: SeriesId("sh1".into()),
        title: "The Expanse".into(),
        sort_title: "expanse".into(),
        year: Some(2015),
        overview: None,
        content_rating: None,
        manually_edited: false,
        added_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
        artwork: Vec::new(),
    });
    catalog.add_season(Season {
        id: SeasonId("se2".into()),
        series: SeriesId("sh1".into()),
        number: 2,
        title: None,
        overview: None,
        added_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
        artwork: Vec::new(),
    });
    catalog.add_episode(Episode {
        id: EpisodeId("e1".into()),
        season: SeasonId("se2".into()),
        number: 4,
        title: "Home".into(),
        overview: None,
        runtime_minutes: None,
        air_date: None,
        manually_edited: false,
        added_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
        artwork: Vec::new(),
    });
    catalog
        .set_title_enrichment(&TitleRef::Series(SeriesId("sh1".into())), &imdb("tt3230854"))
        .await
        .unwrap();
    catalog
}

fn app(catalog: MockCatalogRepo, store: MockStore, provider: MockProvider) -> Router {
    let generator = Generator::new();
    let auth = AppState::new(
        generator.auth.clone(),
        generator.catalog.clone(),
        generator.session.clone(),
        generator.library.clone(),
        generator.user.clone(),
        generator.user_library.clone(),
        generator.discovery.clone(),
        generator.job.clone(),
    );
    subtitle_search_router(auth, SubtitleSearchState::new(catalog, store, provider))
}

async fn get(app: Router, uri: &str, auth: Option<&str>) -> (StatusCode, Vec<u8>) {
    let mut builder = Request::builder().method("GET").uri(uri);
    if let Some(token) = auth {
        builder = builder.header(header::AUTHORIZATION, token);
    }
    let response = app.oneshot(builder.body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap().to_vec();
    (status, body)
}

async fn post(app: Router, uri: &str, auth: Option<&str>, body: &str) -> (StatusCode, Vec<u8>) {
    let mut builder =
        Request::builder().method("POST").uri(uri).header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = auth {
        builder = builder.header(header::AUTHORIZATION, token);
    }
    let response = app.oneshot(builder.body(Body::from(body.to_owned())).unwrap()).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap().to_vec();
    (status, body)
}

const SEARCH: &str = "/api/v1/admin/versions/v1/subtitles/search";
const DOWNLOAD: &str = "/api/v1/admin/versions/v1/subtitles/download";

async fn files(catalog: &MockCatalogRepo) -> Vec<SubtitleFile> {
    catalog.version_detail(&VersionId("v1".into())).await.unwrap().unwrap().subtitle_files
}

#[tokio::test]
async fn an_empty_query_searches_on_the_matched_title() {
    let provider = MockProvider::default();
    let seen = provider.seen.clone();
    let (status, _) =
        get(app(seeded_movie(None).await, MockStore::default(), provider), SEARCH, Some(ADMIN))
            .await;

    assert_eq!(status, StatusCode::OK);
    let query = seen.take();
    assert_eq!(query.query.as_deref(), Some("The Matrix"));
    assert_eq!(query.imdb_id, None);
    assert_eq!(query.season, None);
    assert_eq!(query.episode, None);
}

#[tokio::test]
async fn an_empty_query_prefers_the_imdb_id_over_the_title() {
    let provider = MockProvider::default();
    let seen = provider.seen.clone();
    let (status, _) = get(
        app(seeded_movie(Some("tt0133093")).await, MockStore::default(), provider),
        SEARCH,
        Some(ADMIN),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let query = seen.take();
    assert_eq!(query.imdb_id.as_deref(), Some("tt0133093"));
    assert_eq!(query.query, None);
}

#[tokio::test]
async fn an_episode_carries_its_series_id_season_and_number() {
    let provider = MockProvider::default();
    let seen = provider.seen.clone();
    let (status, _) =
        get(app(seeded_episode().await, MockStore::default(), provider), SEARCH, Some(ADMIN)).await;

    assert_eq!(status, StatusCode::OK);
    let query = seen.take();
    assert_eq!(query.imdb_id.as_deref(), Some("tt3230854"));
    assert_eq!(query.season, Some(2));
    assert_eq!(query.episode, Some(4));
}

#[tokio::test]
async fn an_episode_the_catalog_does_not_have_searches_on_nothing() {
    let catalog = MockCatalogRepo::new();
    catalog.add_version(Version { title: TitleId::Episode(EpisodeId("e1".into())), ..version() });
    let provider = MockProvider::default();
    let seen = provider.seen.clone();

    let (status, _) = get(app(catalog, MockStore::default(), provider), SEARCH, Some(ADMIN)).await;

    assert_eq!(status, StatusCode::OK);
    let query = seen.take();
    assert_eq!(query.imdb_id, None);
    assert_eq!(query.season, None);
    assert_eq!(query.episode, None);
}

#[tokio::test]
async fn an_episode_whose_season_is_gone_still_carries_its_own_number() {
    let catalog = MockCatalogRepo::new();
    catalog.add_version(Version { title: TitleId::Episode(EpisodeId("e1".into())), ..version() });
    catalog.add_episode(Episode {
        id: EpisodeId("e1".into()),
        season: SeasonId("missing".into()),
        number: 4,
        title: "Home".into(),
        overview: None,
        runtime_minutes: None,
        air_date: None,
        manually_edited: false,
        added_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
        artwork: Vec::new(),
    });
    let provider = MockProvider::default();
    let seen = provider.seen.clone();

    let (status, _) = get(app(catalog, MockStore::default(), provider), SEARCH, Some(ADMIN)).await;

    assert_eq!(status, StatusCode::OK);
    let query = seen.take();
    assert_eq!(query.episode, Some(4));
    assert_eq!(query.season, None, "a season that is not there names no number");
    assert_eq!(query.imdb_id, None, "and no series means no imdb id to search on");
}

#[tokio::test]
async fn a_typed_query_replaces_the_matched_title_and_drops_the_imdb_id() {
    let provider = MockProvider::default();
    let seen = provider.seen.clone();
    let (status, _) = get(
        app(seeded_movie(Some("tt0133093")).await, MockStore::default(), provider),
        &format!("{SEARCH}?q=matrix%20reloaded"),
        Some(ADMIN),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let query = seen.take();
    assert_eq!(query.query.as_deref(), Some("matrix reloaded"));
    assert_eq!(query.imdb_id, None);
}

#[tokio::test]
async fn admin_search_returns_mapped_candidates() {
    let catalog = seeded(&[]).await;
    let (status, body) = get(
        app(catalog, MockStore::default(), MockProvider::default()),
        &format!("{SEARCH}?q=matrix&language=en"),
        Some(ADMIN),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let value: Value = serde_json::from_slice(&body).unwrap();
    let items = value.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["file_id"].as_str().unwrap(), "42");
    assert_eq!(items[0]["language"].as_str().unwrap(), "en");
    assert_eq!(items[0]["format"].as_str().unwrap(), "srt");
    assert_eq!(items[0]["release_name"].as_str().unwrap(), "The.Matrix.1999.BluRay");
    assert_eq!(items[0]["download_count"].as_u64().unwrap(), 9999);
    assert!((items[0]["rating"].as_f64().unwrap() - 8.5).abs() < 1e-6);
}

#[traced_test]
#[tokio::test]
async fn search_orders_by_download_count_descending() {
    let catalog = seeded(&[]).await;
    let candidate = |id: &str, downloads: u32| SubtitleCandidate {
        file_id: id.into(),
        language: Some(LanguageCode("en".into())),
        format: SubtitleFormat::Srt,
        release_name: Some(id.into()),
        download_count: Some(downloads),
        rating: None,
    };
    let provider = MockProvider {
        search: SearchMode::Ok(vec![candidate("low", 5), candidate("high", 900)]),
        ..MockProvider::default()
    };
    let (status, body) =
        get(app(catalog, MockStore::default(), provider), SEARCH, Some(ADMIN)).await;
    assert_eq!(status, StatusCode::OK);
    let value: Value = serde_json::from_slice(&body).unwrap();
    let items = value.as_array().unwrap();
    assert_eq!(items[0]["file_id"].as_str().unwrap(), "high");
    assert_eq!(items[1]["file_id"].as_str().unwrap(), "low");
    assert!(logs_contain("user [admin] subtitle search for version [v1]: 2 hits"));
}

#[tokio::test]
async fn search_without_params_still_queries() {
    let catalog = seeded(&[]).await;
    let (status, body) =
        get(app(catalog, MockStore::default(), MockProvider::default()), SEARCH, Some(ADMIN)).await;
    assert_eq!(status, StatusCode::OK);
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn search_not_found_is_empty_list() {
    let catalog = seeded(&[]).await;
    let provider = MockProvider { search: SearchMode::NotFound, ..MockProvider::default() };
    let (status, body) =
        get(app(catalog, MockStore::default(), provider), SEARCH, Some(ADMIN)).await;
    assert_eq!(status, StatusCode::OK);
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert!(value.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn search_provider_backend_error_is_bad_gateway() {
    let catalog = seeded(&[]).await;
    let provider = MockProvider { search: SearchMode::Backend, ..MockProvider::default() };
    let (status, _) = get(app(catalog, MockStore::default(), provider), SEARCH, Some(ADMIN)).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn search_unknown_version_is_not_found() {
    let (status, _) = get(
        app(MockCatalogRepo::new(), MockStore::default(), MockProvider::default()),
        SEARCH,
        Some(ADMIN),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn search_catalog_error_is_internal() {
    let catalog = seeded(&[]).await;
    catalog.set_fail();
    let (status, _) =
        get(app(catalog, MockStore::default(), MockProvider::default()), SEARCH, Some(ADMIN)).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn admin_download_adds_row_keeping_others() {
    let catalog = seeded(&[
        subtitle("sidecar", SubtitleSource::External, "/m/v1.en.srt"),
        subtitle("opensubtitles:v1:99", SubtitleSource::OpenSubtitles, "/subs/v1/99.srt"),
    ])
    .await;

    let (status, _) = post(
        app(catalog.clone(), MockStore::default(), MockProvider::default()),
        DOWNLOAD,
        Some(ADMIN),
        r#"{"file_id":"42","language":"en"}"#,
    )
    .await;

    assert_eq!(status, StatusCode::NO_CONTENT);
    let files = files(&catalog).await;
    assert_eq!(files.len(), 3);
    let added = files.iter().find(|file| file.id.0 == "opensubtitles:v1:42").unwrap();
    assert_eq!(added.source, SubtitleSource::OpenSubtitles);
    assert_eq!(added.language.as_ref().unwrap().0, "en");
    assert!(files.iter().any(|file| file.id.0 == "sidecar"));
    assert!(files.iter().any(|file| file.id.0 == "opensubtitles:v1:99"));
}

#[tokio::test]
async fn downloading_a_file_the_version_already_holds_spends_no_quota() {
    let catalog = seeded(&[subtitle(
        "opensubtitles:v1:42",
        SubtitleSource::OpenSubtitles,
        "/subs/v1/old.srt",
    )])
    .await;
    let provider = MockProvider::default();
    let downloads = provider.downloads.clone();
    let store = MockStore::default();
    let written = store.files.clone();

    let (status, _) =
        post(app(catalog.clone(), store, provider), DOWNLOAD, Some(ADMIN), r#"{"file_id":"42"}"#)
            .await;

    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(downloads.load(Ordering::SeqCst), 0);
    assert!(written.lock().unwrap().is_empty());
    let files = files(&catalog).await;
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "/subs/v1/old.srt");
}

#[tokio::test]
async fn downloading_a_file_the_version_lacks_calls_the_provider_once() {
    let catalog = seeded(&[subtitle(
        "opensubtitles:v1:99",
        SubtitleSource::OpenSubtitles,
        "/subs/v1/99.srt",
    )])
    .await;
    let provider = MockProvider::default();
    let downloads = provider.downloads.clone();

    let (status, _) = post(
        app(catalog.clone(), MockStore::default(), provider),
        DOWNLOAD,
        Some(ADMIN),
        r#"{"file_id":"42"}"#,
    )
    .await;

    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(downloads.load(Ordering::SeqCst), 1);
    let files = files(&catalog).await;
    assert_eq!(files.len(), 2);
    let added = files.iter().find(|file| file.id.0 == "opensubtitles:v1:42").unwrap();
    assert_eq!(added.path, "/subs/v1/42.srt");
    assert!(added.language.is_none());
}

#[tokio::test]
async fn a_hand_picked_download_is_stored_pinned_and_labelled() {
    let catalog = seeded(&[]).await;

    let (status, _) = post(
        app(catalog.clone(), MockStore::default(), MockProvider::default()),
        DOWNLOAD,
        Some(ADMIN),
        r#"{"file_id":"42","language":"en","release_name":"The.Matrix.1999.BluRay"}"#,
    )
    .await;

    assert_eq!(status, StatusCode::NO_CONTENT);
    let files = files(&catalog).await;
    assert_eq!(files[0].label.as_deref(), Some("The.Matrix.1999.BluRay"));
    assert!(files[0].pinned, "an admin's pick must survive the automatic job");
}

#[tokio::test]
async fn a_download_without_a_release_name_is_still_pinned() {
    let catalog = seeded(&[]).await;

    let (status, _) = post(
        app(catalog.clone(), MockStore::default(), MockProvider::default()),
        DOWNLOAD,
        Some(ADMIN),
        r#"{"file_id":"42","release_name":"  "}"#,
    )
    .await;

    assert_eq!(status, StatusCode::NO_CONTENT);
    let files = files(&catalog).await;
    assert_eq!(files[0].label, None);
    assert!(files[0].pinned);
}

#[tokio::test]
async fn download_blank_language_stores_none() {
    let catalog = seeded(&[]).await;
    let (status, _) = post(
        app(catalog.clone(), MockStore::default(), MockProvider::default()),
        DOWNLOAD,
        Some(ADMIN),
        r#"{"file_id":"42","language":"   "}"#,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(files(&catalog).await[0].language.is_none());
}

#[tokio::test]
async fn download_blank_file_id_is_bad_request() {
    let catalog = seeded(&[]).await;
    let (status, _) = post(
        app(catalog, MockStore::default(), MockProvider::default()),
        DOWNLOAD,
        Some(ADMIN),
        r#"{"file_id":"   "}"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn download_provider_not_found_is_not_found() {
    let catalog = seeded(&[]).await;
    let provider = MockProvider { download: DownloadMode::NotFound, ..MockProvider::default() };
    let (status, _) = post(
        app(catalog, MockStore::default(), provider),
        DOWNLOAD,
        Some(ADMIN),
        r#"{"file_id":"42"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn download_provider_backend_error_is_bad_gateway() {
    let catalog = seeded(&[]).await;
    let provider = MockProvider { download: DownloadMode::Backend, ..MockProvider::default() };
    let (status, _) = post(
        app(catalog, MockStore::default(), provider),
        DOWNLOAD,
        Some(ADMIN),
        r#"{"file_id":"42"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn download_store_error_is_internal() {
    let catalog = seeded(&[]).await;
    let (status, _) = post(
        app(catalog, MockStore::failing(), MockProvider::default()),
        DOWNLOAD,
        Some(ADMIN),
        r#"{"file_id":"42"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn download_write_error_is_internal() {
    let catalog = seeded(&[]).await;
    catalog.set_fail_writes();
    let (status, _) = post(
        app(catalog, MockStore::default(), MockProvider::default()),
        DOWNLOAD,
        Some(ADMIN),
        r#"{"file_id":"42"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn download_unknown_version_is_not_found() {
    let (status, _) = post(
        app(MockCatalogRepo::new(), MockStore::default(), MockProvider::default()),
        DOWNLOAD,
        Some(ADMIN),
        r#"{"file_id":"42"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn download_catalog_read_error_is_internal() {
    let catalog = seeded(&[]).await;
    catalog.set_fail();
    let (status, _) = post(
        app(catalog, MockStore::default(), MockProvider::default()),
        DOWNLOAD,
        Some(ADMIN),
        r#"{"file_id":"42"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn non_admin_is_forbidden() {
    let catalog = seeded(&[]).await;
    let (status, _) =
        get(app(catalog, MockStore::default(), MockProvider::default()), SEARCH, Some(USER)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn missing_auth_is_unauthorized() {
    let catalog = seeded(&[]).await;
    let (status, _) =
        get(app(catalog, MockStore::default(), MockProvider::default()), SEARCH, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

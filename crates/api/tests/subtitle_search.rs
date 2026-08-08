use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use tower::ServiceExt;

use api::{AppState, SubtitleSearchState, subtitle_search_router};
use contracts::Generator;
use domain::catalog::{MovieId, TitleId, Version, VersionId};
use domain::common::{LanguageCode, Quality};
use domain::error::SubtitleError;
use domain::library::LibraryId;
use domain::media::{
    FetchedSubtitle, SubtitleCandidate, SubtitleFile, SubtitleFileId, SubtitleFormat,
    SubtitleProvider, SubtitleQuery, SubtitleSource, SubtitleStore,
};
use domain::repository::CatalogRepository;
use jiff::Timestamp;
use services::mock::MockCatalogRepo;

const ADMIN: &str = "Bearer access:admin";
const USER: &str = "Bearer access:u1";

#[derive(Clone, Default)]
struct MockStore {
    files: Arc<Mutex<HashMap<String, String>>>,
    fail: bool,
}

impl MockStore {
    fn failing() -> Self {
        Self {
            fail: true,
            ..Self::default()
        }
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
        self.files
            .lock()
            .unwrap()
            .insert(path.clone(), content.to_owned());
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

struct MockProvider {
    search: SearchMode,
    download: DownloadMode,
}

impl Default for MockProvider {
    fn default() -> Self {
        Self {
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
    async fn search(
        &self,
        _query: &SubtitleQuery,
    ) -> Result<Vec<SubtitleCandidate>, SubtitleError> {
        match &self.search {
            SearchMode::Ok(candidates) => Ok(candidates.clone()),
            SearchMode::NotFound => Err(SubtitleError::NotFound),
            SearchMode::Backend => Err(SubtitleError::Backend("boom".into())),
        }
    }

    async fn download(&self, _file_id: &str) -> Result<FetchedSubtitle, SubtitleError> {
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
        edition: None,
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
    }
}

async fn seeded(files: &[SubtitleFile]) -> MockCatalogRepo {
    let catalog = MockCatalogRepo::new();
    catalog.add_version(version());
    catalog
        .set_subtitle_files(&VersionId("v1".into()), files)
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
    let response = app
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    (status, body)
}

async fn post(app: Router, uri: &str, auth: Option<&str>, body: &str) -> (StatusCode, Vec<u8>) {
    let mut builder = Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = auth {
        builder = builder.header(header::AUTHORIZATION, token);
    }
    let response = app
        .oneshot(builder.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    (status, body)
}

const SEARCH: &str = "/api/v1/admin/versions/v1/subtitles/search";
const DOWNLOAD: &str = "/api/v1/admin/versions/v1/subtitles/download";

async fn files(catalog: &MockCatalogRepo) -> Vec<SubtitleFile> {
    catalog
        .version_detail(&VersionId("v1".into()))
        .await
        .unwrap()
        .unwrap()
        .subtitle_files
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
    assert_eq!(
        items[0]["release_name"].as_str().unwrap(),
        "The.Matrix.1999.BluRay"
    );
    assert_eq!(items[0]["download_count"].as_u64().unwrap(), 9999);
    assert!((items[0]["rating"].as_f64().unwrap() - 8.5).abs() < 1e-6);
}

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
    let (status, body) = get(
        app(catalog, MockStore::default(), provider),
        SEARCH,
        Some(ADMIN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let value: Value = serde_json::from_slice(&body).unwrap();
    let items = value.as_array().unwrap();
    assert_eq!(items[0]["file_id"].as_str().unwrap(), "high");
    assert_eq!(items[1]["file_id"].as_str().unwrap(), "low");
}

#[tokio::test]
async fn search_without_params_still_queries() {
    let catalog = seeded(&[]).await;
    let (status, body) = get(
        app(catalog, MockStore::default(), MockProvider::default()),
        SEARCH,
        Some(ADMIN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn search_not_found_is_empty_list() {
    let catalog = seeded(&[]).await;
    let provider = MockProvider {
        search: SearchMode::NotFound,
        ..MockProvider::default()
    };
    let (status, body) = get(
        app(catalog, MockStore::default(), provider),
        SEARCH,
        Some(ADMIN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert!(value.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn search_provider_backend_error_is_bad_gateway() {
    let catalog = seeded(&[]).await;
    let provider = MockProvider {
        search: SearchMode::Backend,
        ..MockProvider::default()
    };
    let (status, _) = get(
        app(catalog, MockStore::default(), provider),
        SEARCH,
        Some(ADMIN),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn search_unknown_version_is_not_found() {
    let (status, _) = get(
        app(
            MockCatalogRepo::new(),
            MockStore::default(),
            MockProvider::default(),
        ),
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
    let (status, _) = get(
        app(catalog, MockStore::default(), MockProvider::default()),
        SEARCH,
        Some(ADMIN),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn admin_download_adds_row_keeping_others() {
    let catalog = seeded(&[
        subtitle("sidecar", SubtitleSource::External, "/m/v1.en.srt"),
        subtitle(
            "opensubtitles:v1:99",
            SubtitleSource::OpenSubtitles,
            "/subs/v1/99.srt",
        ),
    ])
    .await;

    let (status, _) = post(
        app(
            catalog.clone(),
            MockStore::default(),
            MockProvider::default(),
        ),
        DOWNLOAD,
        Some(ADMIN),
        r#"{"file_id":"42","language":"en"}"#,
    )
    .await;

    assert_eq!(status, StatusCode::NO_CONTENT);
    let files = files(&catalog).await;
    assert_eq!(files.len(), 3);
    let added = files
        .iter()
        .find(|file| file.id.0 == "opensubtitles:v1:42")
        .unwrap();
    assert_eq!(added.source, SubtitleSource::OpenSubtitles);
    assert_eq!(added.language.as_ref().unwrap().0, "en");
    assert!(files.iter().any(|file| file.id.0 == "sidecar"));
    assert!(files.iter().any(|file| file.id.0 == "opensubtitles:v1:99"));
}

#[tokio::test]
async fn download_replaces_a_previous_download_of_the_same_file() {
    let catalog = seeded(&[subtitle(
        "opensubtitles:v1:42",
        SubtitleSource::OpenSubtitles,
        "/subs/v1/old.srt",
    )])
    .await;

    let (status, _) = post(
        app(
            catalog.clone(),
            MockStore::default(),
            MockProvider::default(),
        ),
        DOWNLOAD,
        Some(ADMIN),
        r#"{"file_id":"42"}"#,
    )
    .await;

    assert_eq!(status, StatusCode::NO_CONTENT);
    let files = files(&catalog).await;
    assert_eq!(files.len(), 1);
    let only = &files[0];
    assert_eq!(only.id.0, "opensubtitles:v1:42");
    assert_eq!(only.path, "/subs/v1/42.srt");
    assert!(only.language.is_none());
}

#[tokio::test]
async fn download_blank_language_stores_none() {
    let catalog = seeded(&[]).await;
    let (status, _) = post(
        app(
            catalog.clone(),
            MockStore::default(),
            MockProvider::default(),
        ),
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
    let provider = MockProvider {
        download: DownloadMode::NotFound,
        ..MockProvider::default()
    };
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
    let provider = MockProvider {
        download: DownloadMode::Backend,
        ..MockProvider::default()
    };
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
        app(
            MockCatalogRepo::new(),
            MockStore::default(),
            MockProvider::default(),
        ),
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
    let (status, _) = get(
        app(catalog, MockStore::default(), MockProvider::default()),
        SEARCH,
        Some(USER),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn missing_auth_is_unauthorized() {
    let catalog = seeded(&[]).await;
    let (status, _) = get(
        app(catalog, MockStore::default(), MockProvider::default()),
        SEARCH,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use tower::ServiceExt;

use api::{AppState, SubtitleState, subtitle_router};
use contracts::Generator;
use domain::catalog::{MovieId, TitleId, Version, VersionId};
use domain::common::{LanguageCode, Quality};
use domain::error::SubtitleError;
use domain::library::LibraryId;
use domain::media::{
    SubtitleFile, SubtitleFileId, SubtitleFormat, SubtitleReader, SubtitleSource, SubtitleStore,
};
use domain::repository::CatalogRepository;
use jiff::Timestamp;
use services::mock::MockCatalogRepo;

const ADMIN: &str = "Bearer access:admin";
const USER: &str = "Bearer access:u1";

#[derive(Clone, Default)]
struct MockStore {
    files: Arc<Mutex<HashMap<String, String>>>,
    fail_remove: bool,
}

impl MockStore {
    fn failing_remove() -> Self {
        Self {
            fail_remove: true,
            ..Self::default()
        }
    }
    fn seed(&self, path: &str, content: &str) {
        self.files
            .lock()
            .unwrap()
            .insert(path.to_owned(), content.to_owned());
    }
    fn has(&self, path: &str) -> bool {
        self.files.lock().unwrap().contains_key(path)
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
        let path = format!("/subs/{}/{file_id}.{}", version.0, format.extension());
        self.files
            .lock()
            .unwrap()
            .insert(path.clone(), content.to_owned());
        Ok(path)
    }

    async fn remove(&self, path: &str) -> Result<(), SubtitleError> {
        if self.fail_remove {
            return Err(SubtitleError::Store("boom".into()));
        }
        self.files.lock().unwrap().remove(path);
        Ok(())
    }
}

impl SubtitleReader for MockStore {
    async fn load(&self, path: &str) -> Result<String, SubtitleError> {
        self.files
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| SubtitleError::Backend("missing".into()))
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

fn file(
    id: &str,
    source: SubtitleSource,
    path: &str,
    translated_from: Option<&str>,
) -> SubtitleFile {
    SubtitleFile {
        id: SubtitleFileId(id.into()),
        version: VersionId("v1".into()),
        language: Some(LanguageCode("en".into())),
        format: SubtitleFormat::Vtt,
        source,
        path: path.into(),
        translated_from: translated_from.map(|id| SubtitleFileId(id.into())),
    }
}

async fn seed(files: &[SubtitleFile]) -> (MockCatalogRepo, MockStore) {
    let catalog = MockCatalogRepo::new();
    catalog.add_version(version());
    catalog
        .set_subtitle_files(&VersionId("v1".into()), files)
        .await
        .unwrap();
    (catalog, MockStore::default())
}

fn app(catalog: MockCatalogRepo, store: MockStore) -> Router {
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
    subtitle_router(auth, SubtitleState::new(catalog, store))
}

async fn send(app: Router, method: &str, uri: &str, auth: Option<&str>) -> (StatusCode, Vec<u8>) {
    let mut builder = Request::builder().method(method).uri(uri);
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

async fn send_body(
    app: Router,
    method: &str,
    uri: &str,
    auth: Option<&str>,
    body: &str,
) -> (StatusCode, Vec<u8>) {
    let mut builder = Request::builder()
        .method(method)
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

fn uri(subtitle: &str) -> String {
    format!("/api/v1/admin/versions/v1/subtitles/{subtitle}")
}

async fn language_of(catalog: &MockCatalogRepo, subtitle: &str) -> Option<String> {
    catalog
        .version_detail(&VersionId("v1".into()))
        .await
        .unwrap()
        .unwrap()
        .subtitle_files
        .into_iter()
        .find(|file| file.id.0 == subtitle)
        .unwrap()
        .language
        .map(|code| code.0)
}

#[tokio::test]
async fn admin_views_subtitle_text() {
    let (catalog, store) = seed(&[file(
        "generated:v1",
        SubtitleSource::Generated,
        "/subs/v1/gen.vtt",
        None,
    )])
    .await;
    store.seed("/subs/v1/gen.vtt", "WEBVTT\n\nhello\n");

    let (status, body) = send(
        app(catalog, store),
        "GET",
        &uri("generated:v1"),
        Some(ADMIN),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["content"].as_str().unwrap(), "WEBVTT\n\nhello\n");
}

#[tokio::test]
async fn view_unknown_subtitle_is_not_found() {
    let (catalog, store) = seed(&[]).await;
    let (status, _) = send(app(catalog, store), "GET", &uri("nope"), Some(ADMIN)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn view_unknown_version_is_not_found() {
    let catalog = MockCatalogRepo::new();
    let (status, _) = send(
        app(catalog, MockStore::default()),
        "GET",
        &uri("generated:v1"),
        Some(ADMIN),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn admin_deletes_generated_subtitle_and_file() {
    let (catalog, store) = seed(&[file(
        "generated:v1",
        SubtitleSource::Generated,
        "/subs/v1/gen.vtt",
        None,
    )])
    .await;
    store.seed("/subs/v1/gen.vtt", "WEBVTT\n\nhello\n");

    let (status, _) = send(
        app(catalog.clone(), store.clone()),
        "DELETE",
        &uri("generated:v1"),
        Some(ADMIN),
    )
    .await;

    assert_eq!(status, StatusCode::NO_CONTENT);
    let remaining = catalog
        .version_detail(&VersionId("v1".into()))
        .await
        .unwrap()
        .unwrap()
        .subtitle_files;
    assert!(remaining.is_empty());
    assert!(!store.has("/subs/v1/gen.vtt"));
}

#[tokio::test]
async fn deleting_a_source_prunes_orphaned_translations() {
    let (catalog, store) = seed(&[
        file(
            "opensubtitles:v1:42",
            SubtitleSource::OpenSubtitles,
            "/subs/v1/os.srt",
            None,
        ),
        file(
            "machine:v1:fr",
            SubtitleSource::MachineTranslated,
            "/subs/v1/fr.vtt",
            Some("opensubtitles:v1:42"),
        ),
    ])
    .await;
    store.seed("/subs/v1/os.srt", "1\nhi\n");
    store.seed("/subs/v1/fr.vtt", "WEBVTT\n\nbonjour\n");

    let (status, _) = send(
        app(catalog.clone(), store.clone()),
        "DELETE",
        &uri("opensubtitles:v1:42"),
        Some(ADMIN),
    )
    .await;

    assert_eq!(status, StatusCode::NO_CONTENT);
    let remaining = catalog
        .version_detail(&VersionId("v1".into()))
        .await
        .unwrap()
        .unwrap()
        .subtitle_files;
    assert!(remaining.is_empty());
    assert!(!store.has("/subs/v1/os.srt"));
    assert!(!store.has("/subs/v1/fr.vtt"));
}

#[tokio::test]
async fn deleting_an_external_sidecar_is_forbidden() {
    let (catalog, store) = seed(&[file(
        "sidecar",
        SubtitleSource::External,
        "/m/v1.en.srt",
        None,
    )])
    .await;

    let (status, _) = send(
        app(catalog.clone(), store),
        "DELETE",
        &uri("sidecar"),
        Some(ADMIN),
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(
        catalog
            .version_detail(&VersionId("v1".into()))
            .await
            .unwrap()
            .unwrap()
            .subtitle_files
            .len(),
        1
    );
}

#[tokio::test]
async fn delete_unknown_subtitle_is_not_found() {
    let (catalog, store) = seed(&[]).await;
    let (status, _) = send(app(catalog, store), "DELETE", &uri("nope"), Some(ADMIN)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn view_catalog_error_is_internal() {
    let (catalog, store) = seed(&[file(
        "generated:v1",
        SubtitleSource::Generated,
        "/subs/v1/gen.vtt",
        None,
    )])
    .await;
    catalog.set_fail();
    let (status, _) = send(
        app(catalog, store),
        "GET",
        &uri("generated:v1"),
        Some(ADMIN),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn view_missing_file_is_internal() {
    let (catalog, store) = seed(&[file(
        "generated:v1",
        SubtitleSource::Generated,
        "/subs/v1/gone.vtt",
        None,
    )])
    .await;
    let (status, _) = send(
        app(catalog, store),
        "GET",
        &uri("generated:v1"),
        Some(ADMIN),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn delete_catalog_error_is_internal() {
    let (catalog, store) = seed(&[file(
        "generated:v1",
        SubtitleSource::Generated,
        "/subs/v1/gen.vtt",
        None,
    )])
    .await;
    catalog.set_fail();
    let (status, _) = send(
        app(catalog, store),
        "DELETE",
        &uri("generated:v1"),
        Some(ADMIN),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn delete_succeeds_even_if_file_removal_fails() {
    let (catalog, _) = seed(&[file(
        "generated:v1",
        SubtitleSource::Generated,
        "/subs/v1/gen.vtt",
        None,
    )])
    .await;
    let store = MockStore::failing_remove();
    store.seed("/subs/v1/gen.vtt", "WEBVTT\n\nhello\n");

    let (status, _) = send(
        app(catalog.clone(), store),
        "DELETE",
        &uri("generated:v1"),
        Some(ADMIN),
    )
    .await;

    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(
        catalog
            .version_detail(&VersionId("v1".into()))
            .await
            .unwrap()
            .unwrap()
            .subtitle_files
            .is_empty()
    );
}

#[tokio::test]
async fn admin_renames_a_generated_subtitle_and_leaves_others() {
    let (catalog, store) = seed(&[
        file(
            "generated:v1",
            SubtitleSource::Generated,
            "/subs/v1/gen.vtt",
            None,
        ),
        file(
            "opensubtitles:v1:42",
            SubtitleSource::OpenSubtitles,
            "/subs/v1/os.srt",
            None,
        ),
    ])
    .await;

    let (status, _) = send_body(
        app(catalog.clone(), store),
        "PUT",
        &uri("generated:v1"),
        Some(ADMIN),
        r#"{"language":"fr"}"#,
    )
    .await;

    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        language_of(&catalog, "generated:v1").await,
        Some("fr".to_owned())
    );
    assert_eq!(
        language_of(&catalog, "opensubtitles:v1:42").await,
        Some("en".to_owned())
    );
}

#[tokio::test]
async fn renaming_to_blank_clears_the_language() {
    let (catalog, store) = seed(&[file(
        "generated:v1",
        SubtitleSource::Generated,
        "/subs/v1/gen.vtt",
        None,
    )])
    .await;

    let (status, _) = send_body(
        app(catalog.clone(), store),
        "PUT",
        &uri("generated:v1"),
        Some(ADMIN),
        r#"{"language":"  "}"#,
    )
    .await;

    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(language_of(&catalog, "generated:v1").await, None);
}

#[tokio::test]
async fn renaming_an_external_sidecar_is_forbidden() {
    let (catalog, store) = seed(&[file(
        "sidecar",
        SubtitleSource::External,
        "/m/v1.en.srt",
        None,
    )])
    .await;

    let (status, _) = send_body(
        app(catalog, store),
        "PUT",
        &uri("sidecar"),
        Some(ADMIN),
        r#"{"language":"fr"}"#,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn rename_unknown_subtitle_is_not_found() {
    let (catalog, store) = seed(&[]).await;
    let (status, _) = send_body(
        app(catalog, store),
        "PUT",
        &uri("nope"),
        Some(ADMIN),
        r#"{"language":"fr"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn rename_unknown_version_is_not_found() {
    let catalog = MockCatalogRepo::new();
    let (status, _) = send_body(
        app(catalog, MockStore::default()),
        "PUT",
        &uri("generated:v1"),
        Some(ADMIN),
        r#"{"language":"fr"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn rename_read_error_is_internal() {
    let (catalog, store) = seed(&[file(
        "generated:v1",
        SubtitleSource::Generated,
        "/subs/v1/gen.vtt",
        None,
    )])
    .await;
    catalog.set_fail();
    let (status, _) = send_body(
        app(catalog, store),
        "PUT",
        &uri("generated:v1"),
        Some(ADMIN),
        r#"{"language":"fr"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn rename_write_error_is_internal() {
    let (catalog, store) = seed(&[file(
        "generated:v1",
        SubtitleSource::Generated,
        "/subs/v1/gen.vtt",
        None,
    )])
    .await;
    catalog.set_fail_writes();
    let (status, _) = send_body(
        app(catalog, store),
        "PUT",
        &uri("generated:v1"),
        Some(ADMIN),
        r#"{"language":"fr"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn non_admin_is_forbidden() {
    let (catalog, store) = seed(&[]).await;
    let (status, _) = send(app(catalog, store), "GET", &uri("generated:v1"), Some(USER)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn missing_auth_is_unauthorized() {
    let (catalog, store) = seed(&[]).await;
    let (status, _) = send(app(catalog, store), "GET", &uri("generated:v1"), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

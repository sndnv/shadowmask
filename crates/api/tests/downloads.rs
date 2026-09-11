use std::path::Path;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode, header};
use serde_json::Value;
use tower::ServiceExt;

use api::{AppState, DownloadState, download_router};
use contracts::Generator;
use domain::catalog::{MovieId, TitleId, Version, VersionId};
use domain::common::Quality;
use domain::library::LibraryId;
use domain::session::{DownloadClaims, DownloadTokens};
use jiff::Timestamp;
use mocks::{MockCatalogRepo, MockDownloadTokens};

const USER: &str = "Bearer access:u1";
const PAYLOAD: &[u8] = b"NOT-REALLY-A-MOVIE-BUT-CLOSE-ENOUGH";

fn version_at(id: &str, path: &str, available: bool) -> Version {
    Version {
        id: VersionId(id.into()),
        title: TitleId::Movie(MovieId("m1".into())),
        library: LibraryId("lib1".into()),
        quality: Quality::Fhd,
        container: "mkv".into(),
        path: path.into(),
        size_bytes: PAYLOAD.len() as u64,
        duration_ms: 1000,
        available,
        added_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
    }
}

struct Harness {
    app: Router,
    tokens: MockDownloadTokens,
}

fn harness(extra: &[Version], repo_versions: &[Version]) -> Harness {
    let generator = Generator::new();
    for version in extra {
        generator.catalog_repo.add_version(version.clone());
    }
    let state = AppState::new(
        generator.auth.clone(),
        generator.catalog.clone(),
        generator.session.clone(),
        generator.library.clone(),
        generator.user.clone(),
        generator.user_library.clone(),
        generator.discovery.clone(),
        generator.job.clone(),
    );
    let repo = MockCatalogRepo::new();
    for version in repo_versions {
        repo.add_version(version.clone());
    }
    let tokens = MockDownloadTokens::new();
    let app = download_router(state.clone(), DownloadState::new(state, repo, tokens.clone()));
    Harness { app, tokens }
}

fn seeded_file(dir: &Path, name: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, PAYLOAD).unwrap();
    path.to_string_lossy().into_owned()
}

async fn send(
    app: Router,
    method: &str,
    uri: &str,
    headers: &[(header::HeaderName, &str)],
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut builder = Request::builder().method(method).uri(uri);
    for (name, value) in headers {
        builder = builder.header(name, *value);
    }
    let response = app.oneshot(builder.body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status();
    let response_headers = response.headers().clone();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap().to_vec();
    (status, response_headers, body)
}

fn json(body: &[u8]) -> Value {
    serde_json::from_slice(body).unwrap()
}

fn header_str(headers: &HeaderMap, name: header::HeaderName) -> &str {
    headers.get(name).unwrap().to_str().unwrap()
}

#[tokio::test]
async fn a_link_names_the_file_and_carries_a_token_for_that_version() {
    let harness = harness(&[], &[]);
    let (status, _, body) =
        send(harness.app, "POST", "/api/v1/versions/v1/download", &[(header::AUTHORIZATION, USER)])
            .await;

    assert_eq!(status, StatusCode::OK);
    let body = json(&body);
    assert_eq!(body["filename"], "v1.mkv");
    assert_eq!(body["size_bytes"], 1);
    let url = body["url"].as_str().unwrap();
    let token = url.strip_prefix("/download/").expect("url is a download url");
    let claims = harness.tokens.verify(token).unwrap();
    assert_eq!(claims.version.0, "v1");
    assert_eq!(claims.user.0, "u1");
    assert!(!body["expires_at"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn a_link_never_reveals_the_source_path() {
    let harness = harness(&[], &[]);
    let (_, _, body) =
        send(harness.app, "POST", "/api/v1/versions/v1/download", &[(header::AUTHORIZATION, USER)])
            .await;

    let text = String::from_utf8(body).unwrap();
    assert!(!text.contains("/media/"), "the response leaked a filesystem path: {text}");
}

#[tokio::test]
async fn a_link_requires_authentication() {
    let harness = harness(&[], &[]);
    let (status, _, _) = send(harness.app, "POST", "/api/v1/versions/v1/download", &[]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_link_for_an_unknown_version_is_not_found() {
    let harness = harness(&[], &[]);
    let (status, _, _) = send(
        harness.app,
        "POST",
        "/api/v1/versions/nope/download",
        &[(header::AUTHORIZATION, USER)],
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_link_for_an_unavailable_version_is_not_found() {
    let gone = version_at("gone", "/media/gone.mkv", false);
    let harness = harness(std::slice::from_ref(&gone), &[]);
    let (status, _, _) = send(
        harness.app,
        "POST",
        "/api/v1/versions/gone/download",
        &[(header::AUTHORIZATION, USER)],
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_token_downloads_the_file_without_an_authorization_header() {
    let dir = tempfile::tempdir().unwrap();
    let path = seeded_file(dir.path(), "Big Buck Bunny.mkv");
    let version = version_at("v9", &path, true);
    let harness = harness(&[], std::slice::from_ref(&version));
    let token = harness
        .tokens
        .create(&DownloadClaims {
            user: domain::user::UserId("u1".into()),
            version: VersionId("v9".into()),
            expires_at: Timestamp::from_second(Timestamp::now().as_second() + 3600).unwrap(),
            nonce: "n1".into(),
        })
        .unwrap();

    let (status, headers, body) =
        send(harness.app, "GET", &format!("/download/{}", token.0), &[]).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        header_str(&headers, header::CONTENT_DISPOSITION),
        "attachment; filename=\"Big Buck Bunny.mkv\""
    );
    assert_eq!(body, PAYLOAD);
}

#[tokio::test]
async fn a_download_supports_range_requests() {
    let dir = tempfile::tempdir().unwrap();
    let path = seeded_file(dir.path(), "clip.mkv");
    let version = version_at("v9", &path, true);
    let harness = harness(&[], std::slice::from_ref(&version));
    let token = harness
        .tokens
        .create(&DownloadClaims {
            user: domain::user::UserId("u1".into()),
            version: VersionId("v9".into()),
            expires_at: Timestamp::from_second(Timestamp::now().as_second() + 3600).unwrap(),
            nonce: "n1".into(),
        })
        .unwrap();

    let (status, _, body) = send(
        harness.app,
        "GET",
        &format!("/download/{}", token.0),
        &[(header::RANGE, "bytes=0-3")],
    )
    .await;

    assert_eq!(status, StatusCode::PARTIAL_CONTENT, "a paused download must be resumable");
    assert_eq!(body, &PAYLOAD[0..4]);
}

#[tokio::test]
async fn a_garbage_token_is_refused() {
    let harness = harness(&[], &[]);
    let (status, _, _) = send(harness.app, "GET", "/download/not-a-token", &[]).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn an_expired_token_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = seeded_file(dir.path(), "clip.mkv");
    let version = version_at("v9", &path, true);
    let harness = harness(&[], std::slice::from_ref(&version));
    let expired = format!("u1~v9~{}~n1", Timestamp::now().as_second() - 60);

    let (status, _, _) = send(harness.app, "GET", &format!("/download/{expired}"), &[]).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_token_for_a_deleted_version_is_not_found() {
    let harness = harness(&[], &[]);
    let token = harness
        .tokens
        .create(&DownloadClaims {
            user: domain::user::UserId("u1".into()),
            version: VersionId("ghost".into()),
            expires_at: Timestamp::from_second(Timestamp::now().as_second() + 3600).unwrap(),
            nonce: "n1".into(),
        })
        .unwrap();

    let (status, _, _) = send(harness.app, "GET", &format!("/download/{}", token.0), &[]).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_token_whose_file_vanished_is_not_found() {
    let version = version_at("v9", "/nowhere/gone.mkv", true);
    let harness = harness(&[], std::slice::from_ref(&version));
    let token = harness
        .tokens
        .create(&DownloadClaims {
            user: domain::user::UserId("u1".into()),
            version: VersionId("v9".into()),
            expires_at: Timestamp::from_second(Timestamp::now().as_second() + 3600).unwrap(),
            nonce: "n1".into(),
        })
        .unwrap();

    let (status, _, _) = send(harness.app, "GET", &format!("/download/{}", token.0), &[]).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_token_for_an_unavailable_version_is_not_found() {
    let version = version_at("v9", "/media/v9.mkv", false);
    let harness = harness(&[], std::slice::from_ref(&version));
    let token = harness
        .tokens
        .create(&DownloadClaims {
            user: domain::user::UserId("u1".into()),
            version: VersionId("v9".into()),
            expires_at: Timestamp::from_second(Timestamp::now().as_second() + 3600).unwrap(),
            nonce: "n1".into(),
        })
        .unwrap();

    let (status, _, _) = send(harness.app, "GET", &format!("/download/{}", token.0), &[]).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_failing_codec_is_an_internal_error() {
    let harness = harness(&[], &[]);
    harness.tokens.set_fail();
    let (status, _, _) =
        send(harness.app, "POST", "/api/v1/versions/v1/download", &[(header::AUTHORIZATION, USER)])
            .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

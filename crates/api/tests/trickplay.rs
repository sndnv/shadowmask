use std::path::Path;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode, header};
use tower::ServiceExt;

use api::{AppState, TrickplayState, trickplay_router};
use contracts::Generator;

const VERSION_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
const JPEG_BYTES: &[u8] = b"\xff\xd8\xff\xe0FAKE-JPEG-PAYLOAD";
const BEARER: &str = "Bearer access:u1";

fn app(dir: &Path) -> Router {
    let sheet_dir = dir.join(VERSION_ID);
    std::fs::create_dir_all(&sheet_dir).unwrap();
    std::fs::write(sheet_dir.join("sheet-000.jpg"), JPEG_BYTES).unwrap();
    let generator = Generator::new();
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
    trickplay_router(state, TrickplayState::new(dir))
}

async fn send(
    app: Router,
    uri: &str,
    headers: &[(header::HeaderName, &str)],
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut builder = Request::builder().method("GET").uri(uri);
    for (name, value) in headers {
        builder = builder.header(name, *value);
    }
    let response = app.oneshot(builder.body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status();
    let response_headers = response.headers().clone();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap().to_vec();
    (status, response_headers, body)
}

fn header_str(headers: &HeaderMap, name: header::HeaderName) -> &str {
    headers.get(name).unwrap().to_str().unwrap()
}

#[tokio::test]
async fn jpeg_served_with_cache_headers_and_body() {
    let dir = tempfile::tempdir().unwrap();
    let uri = format!("/api/v1/trickplay/{VERSION_ID}/0");
    let (status, headers, body) =
        send(app(dir.path()), &uri, &[(header::AUTHORIZATION, BEARER)]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(header_str(&headers, header::CONTENT_TYPE), "image/jpeg");
    assert_eq!(header_str(&headers, header::CACHE_CONTROL), "public, max-age=31536000, immutable");
    assert_eq!(header_str(&headers, header::ETAG), format!("\"{VERSION_ID}-0\""));
    assert_eq!(body, JPEG_BYTES);
}

#[tokio::test]
async fn missing_sheet_is_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let uri = format!("/api/v1/trickplay/{VERSION_ID}/9");
    let (status, _, _) = send(app(dir.path()), &uri, &[(header::AUTHORIZATION, BEARER)]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn invalid_version_id_is_bad_request() {
    let dir = tempfile::tempdir().unwrap();
    let (status, _, _) = send(
        app(dir.path()),
        "/api/v1/trickplay/not_hex_zzz/0",
        &[(header::AUTHORIZATION, BEARER)],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn matching_etag_returns_not_modified() {
    let dir = tempfile::tempdir().unwrap();
    let uri = format!("/api/v1/trickplay/{VERSION_ID}/0");
    let etag = format!("\"{VERSION_ID}-0\"");
    let (status, headers, body) = send(
        app(dir.path()),
        &uri,
        &[(header::AUTHORIZATION, BEARER), (header::IF_NONE_MATCH, &etag)],
    )
    .await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert_eq!(header_str(&headers, header::ETAG), etag);
    assert_eq!(header_str(&headers, header::CACHE_CONTROL), "public, max-age=31536000, immutable");
    assert!(body.is_empty());
}

#[tokio::test]
async fn missing_token_is_unauthorized() {
    let dir = tempfile::tempdir().unwrap();
    let uri = format!("/api/v1/trickplay/{VERSION_ID}/0");
    let (status, _, _) = send(app(dir.path()), &uri, &[]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

use std::path::Path;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode, header};
use tower::ServiceExt;

use api::{ImageState, image_router};

const ARTWORK_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
const MISSING_ID: &str = "ffffffff-ffff-ffff-ffff-ffffffffffff";
const PNG_BYTES: &[u8] = b"\x89PNG\r\n\x1a\nFAKE-PNG-PAYLOAD";
const JPEG_BYTES: &[u8] = b"\xff\xd8\xffFAKE-JPEG-PAYLOAD";

fn app_serving(dir: &Path, file: &str, bytes: &[u8]) -> Router {
    let art_dir = dir.join(ARTWORK_ID);
    std::fs::create_dir_all(&art_dir).unwrap();
    std::fs::write(art_dir.join(file), bytes).unwrap();
    image_router(ImageState::new(dir))
}

fn app(dir: &Path) -> Router {
    app_serving(dir, "480.png", PNG_BYTES)
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
    let response = app
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let response_headers = response.headers().clone();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    (status, response_headers, body)
}

fn header_str(headers: &HeaderMap, name: header::HeaderName) -> &str {
    headers.get(name).unwrap().to_str().unwrap()
}

#[tokio::test]
async fn png_served_with_cache_headers_and_body() {
    let dir = tempfile::tempdir().unwrap();
    let uri = format!("/images/{ARTWORK_ID}/480");
    let (status, headers, body) = send(app(dir.path()), &uri, &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(header_str(&headers, header::CONTENT_TYPE), "image/png");
    assert_eq!(
        header_str(&headers, header::CACHE_CONTROL),
        "public, max-age=31536000, immutable"
    );
    assert_eq!(
        header_str(&headers, header::ETAG),
        format!("\"{ARTWORK_ID}-480\"")
    );
    assert_eq!(body, PNG_BYTES);
}

#[tokio::test]
async fn a_jpeg_rung_is_served_as_jpeg() {
    let dir = tempfile::tempdir().unwrap();
    let uri = format!("/images/{ARTWORK_ID}/480");
    let (status, headers, body) =
        send(app_serving(dir.path(), "480.jpg", JPEG_BYTES), &uri, &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(header_str(&headers, header::CONTENT_TYPE), "image/jpeg");
    assert_eq!(
        header_str(&headers, header::CACHE_CONTROL),
        "public, max-age=31536000, immutable"
    );
    assert_eq!(body, JPEG_BYTES);
}

#[tokio::test]
async fn a_transparent_rung_stored_as_png_is_still_served_as_png() {
    let dir = tempfile::tempdir().unwrap();
    let uri = format!("/images/{ARTWORK_ID}/480");
    let (status, headers, body) =
        send(app_serving(dir.path(), "480.png", PNG_BYTES), &uri, &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(header_str(&headers, header::CONTENT_TYPE), "image/png");
    assert_eq!(body, PNG_BYTES);
}

#[tokio::test]
async fn unknown_id_is_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let uri = format!("/images/{MISSING_ID}/480");
    let (status, _, _) = send(app(dir.path()), &uri, &[]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn unsupported_width_is_bad_request() {
    let dir = tempfile::tempdir().unwrap();
    let uri = format!("/images/{ARTWORK_ID}/500");
    let (status, _, _) = send(app(dir.path()), &uri, &[]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn non_numeric_width_is_bad_request() {
    let dir = tempfile::tempdir().unwrap();
    let uri = format!("/images/{ARTWORK_ID}/wide");
    let (status, _, _) = send(app(dir.path()), &uri, &[]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn invalid_id_is_bad_request() {
    let dir = tempfile::tempdir().unwrap();
    let (status, _, _) = send(app(dir.path()), "/images/not_hex_zzz/480", &[]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn matching_if_none_match_is_not_modified() {
    let dir = tempfile::tempdir().unwrap();
    let uri = format!("/images/{ARTWORK_ID}/480");
    let etag = format!("\"{ARTWORK_ID}-480\"");
    let (status, headers, body) =
        send(app(dir.path()), &uri, &[(header::IF_NONE_MATCH, &etag)]).await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert_eq!(header_str(&headers, header::ETAG), etag);
    assert_eq!(
        header_str(&headers, header::CACHE_CONTROL),
        "public, max-age=31536000, immutable"
    );
    assert!(body.is_empty());
}

#[tokio::test]
async fn non_matching_if_none_match_serves_content() {
    let dir = tempfile::tempdir().unwrap();
    let uri = format!("/images/{ARTWORK_ID}/480");
    let (status, _, body) = send(
        app(dir.path()),
        &uri,
        &[(header::IF_NONE_MATCH, "\"stale\"")],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, PNG_BYTES);
}

#[tokio::test]
async fn range_returns_partial_content() {
    let dir = tempfile::tempdir().unwrap();
    let uri = format!("/images/{ARTWORK_ID}/480");
    let (status, headers, body) =
        send(app(dir.path()), &uri, &[(header::RANGE, "bytes=0-3")]).await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert!(headers.contains_key(header::CONTENT_RANGE));
    assert_eq!(body, &PNG_BYTES[0..4]);
}

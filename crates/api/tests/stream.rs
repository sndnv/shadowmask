use std::path::{Path, PathBuf};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode, header};
use jiff::Timestamp;
use tower::ServiceExt;

use api::{StreamState, stream_router};
use domain::catalog::VersionId;
use domain::error::{StreamError, StreamTokenError};
use domain::session::{SessionId, StreamClaims, StreamSource, StreamToken, StreamTokens};
use domain::user::UserId;

fn claims(session: &str) -> StreamClaims {
    StreamClaims {
        session: SessionId(session.to_owned()),
        user: UserId("u1".to_owned()),
        version: VersionId("ver-1".to_owned()),
        expires_at: Timestamp::now(),
    }
}

struct FakeTokens;

impl StreamTokens for FakeTokens {
    fn create(&self, claims: &StreamClaims) -> Result<StreamToken, StreamTokenError> {
        Ok(StreamToken(format!("good:{}", claims.session.0)))
    }

    fn verify(&self, token: &str) -> Result<StreamClaims, StreamTokenError> {
        if token == "expired" {
            Err(StreamTokenError::Expired)
        } else if let Some(session) = token.strip_prefix("good:") {
            Ok(claims(session))
        } else {
            Err(StreamTokenError::Invalid)
        }
    }
}

struct FakeSource {
    output_dir: PathBuf,
    direct_path: PathBuf,
}

impl FakeSource {
    fn ensure_live(&self, claims: &StreamClaims) -> Result<(), StreamError> {
        if claims.session.0 == "live" {
            Ok(())
        } else {
            Err(StreamError::NotLive)
        }
    }
}

impl StreamSource for FakeSource {
    fn master_playlist(&self, claims: &StreamClaims) -> Result<String, StreamError> {
        self.ensure_live(claims)?;
        Ok("#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1\nv0/index.m3u8\n".to_owned())
    }

    fn media_path(
        &self,
        claims: &StreamClaims,
        variant: &str,
        file: &str,
    ) -> Result<PathBuf, StreamError> {
        self.ensure_live(claims)?;
        if variant == "bad" {
            return Err(StreamError::Invalid);
        }
        Ok(self.output_dir.join(file))
    }

    fn direct_file(&self, claims: &StreamClaims) -> Result<PathBuf, StreamError> {
        self.ensure_live(claims)?;
        Ok(self.direct_path.clone())
    }
}

fn app(dir: &Path) -> Router {
    let output_dir = dir.join("out");
    std::fs::create_dir_all(&output_dir).unwrap();
    std::fs::write(output_dir.join("seg_00001.ts"), b"SEGMENT-BYTES").unwrap();
    std::fs::write(output_dir.join("index.m3u8"), b"#EXTM3U\nmedia").unwrap();
    std::fs::write(output_dir.join("subs.vtt"), b"WEBVTT\n").unwrap();
    std::fs::write(output_dir.join("data.bin"), b"\x00\x01\x02\x03").unwrap();
    let direct_path = dir.join("movie.mkv");
    std::fs::write(&direct_path, b"MKVDATA-DIRECT-PLAY").unwrap();
    stream_router(StreamState::new(
        FakeTokens,
        FakeSource {
            output_dir,
            direct_path,
        },
    ))
}

async fn send(app: Router, uri: &str, range: Option<&str>) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut builder = Request::builder().method("GET").uri(uri);
    if let Some(range) = range {
        builder = builder.header(header::RANGE, range);
    }
    let response = app
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    (status, headers, body)
}

fn content_type(headers: &HeaderMap) -> &str {
    headers.get(header::CONTENT_TYPE).unwrap().to_str().unwrap()
}

#[tokio::test]
async fn master_playlist_served_with_m3u8_content_type() {
    let dir = tempfile::tempdir().unwrap();
    let (status, headers, body) =
        send(app(dir.path()), "/stream/good:live/master.m3u8", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type(&headers), "application/vnd.apple.mpegurl");
    assert!(body.starts_with(b"#EXTM3U"));
}

#[tokio::test]
async fn segment_served_with_content_type_and_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let (status, headers, body) =
        send(app(dir.path()), "/stream/good:live/v0/seg_00001.ts", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type(&headers), "video/mp2t");
    assert_eq!(body, b"SEGMENT-BYTES");
}

#[tokio::test]
async fn media_playlist_served_with_m3u8_content_type() {
    let dir = tempfile::tempdir().unwrap();
    let (status, headers, _) = send(app(dir.path()), "/stream/good:live/v0/index.m3u8", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type(&headers), "application/vnd.apple.mpegurl");
}

#[tokio::test]
async fn vtt_served_with_text_vtt_content_type() {
    let dir = tempfile::tempdir().unwrap();
    let (status, headers, _) = send(app(dir.path()), "/stream/good:live/subs/subs.vtt", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type(&headers), "text/vtt");
}

#[tokio::test]
async fn unknown_extension_is_not_overridden() {
    let dir = tempfile::tempdir().unwrap();
    let (status, headers, body) =
        send(app(dir.path()), "/stream/good:live/v0/data.bin", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_ne!(content_type(&headers), "video/mp2t");
    assert_eq!(body, b"\x00\x01\x02\x03");
}

#[tokio::test]
async fn segment_range_returns_206_partial() {
    let dir = tempfile::tempdir().unwrap();
    let (status, headers, body) = send(
        app(dir.path()),
        "/stream/good:live/v0/seg_00001.ts",
        Some("bytes=0-3"),
    )
    .await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert!(headers.contains_key(header::CONTENT_RANGE));
    assert_eq!(body, b"SEGM");
}

#[tokio::test]
async fn direct_file_served_with_and_without_range() {
    let dir = tempfile::tempdir().unwrap();
    let (status, _, body) = send(app(dir.path()), "/stream/good:live/file", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"MKVDATA-DIRECT-PLAY");

    let (status, headers, _) =
        send(app(dir.path()), "/stream/good:live/file", Some("bytes=0-2")).await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert!(headers.contains_key(header::CONTENT_RANGE));
}

#[tokio::test]
async fn expired_token_is_forbidden() {
    let dir = tempfile::tempdir().unwrap();
    let (status, _, _) = send(app(dir.path()), "/stream/expired/master.m3u8", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn garbage_token_is_forbidden() {
    let dir = tempfile::tempdir().unwrap();
    let (status, _, _) = send(app(dir.path()), "/stream/whatever/master.m3u8", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn dead_session_is_forbidden() {
    let dir = tempfile::tempdir().unwrap();
    let (status, _, _) = send(app(dir.path()), "/stream/good:dead/master.m3u8", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn bad_path_is_bad_request() {
    let dir = tempfile::tempdir().unwrap();
    let (status, _, _) = send(app(dir.path()), "/stream/good:live/bad/index.m3u8", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn missing_file_is_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let (status, _, _) = send(app(dir.path()), "/stream/good:live/v0/missing.ts", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

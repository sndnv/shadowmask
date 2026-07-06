use std::path::{Path, PathBuf};
use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode, header};
use jiff::Timestamp;
use tower::ServiceExt;

use api::{StreamState, stream_router};
use domain::catalog::VersionId;
use domain::media::MediaProbe;
use domain::session::{
    DeliveryMode, SessionId, StreamClaims, StreamRegistration, StreamRegistry, StreamTokens,
    TranscodeManager, TranscodeSpec,
};
use domain::user::UserId;
use media::hls::HlsStreamSource;
use media::probe::FfprobeMediaProbe;
use media::stream_token::HmacStreamTokens;
use media::transcode::FfmpegTranscodeManager;

const SECRET: &[u8] = b"shadowmask-smoke-secret";
const SESSION: &str = "smoke-session";

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../media/tests/fixtures/sample_real_bbb.mp4")
        .canonicalize()
        .expect("fixture sample_real_bbb.mp4 should exist")
}

fn first_segment(variant_dir: &Path) -> Option<String> {
    let mut names: Vec<String> = std::fs::read_dir(variant_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("seg_") && name.ends_with(".ts"))
        .collect();
    names.sort();
    names.into_iter().find(|name| {
        std::fs::metadata(variant_dir.join(name))
            .map(|meta| meta.len() > 0)
            .unwrap_or(false)
    })
}

async fn wait_for_completed_segment(variant_dir: &Path) -> String {
    let playlist = variant_dir.join("index.m3u8");
    for _ in 0..300 {
        let finalized = std::fs::read_to_string(&playlist)
            .map(|text| text.contains("#EXT-X-ENDLIST"))
            .unwrap_or(false);
        if finalized && let Some(name) = first_segment(variant_dir) {
            return name;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("ffmpeg did not finalize an HLS segment within the timeout");
}

async fn get(app: Router, uri: &str, range: Option<&str>) -> (StatusCode, HeaderMap, Vec<u8>) {
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

#[tokio::test]
async fn transcodes_fixture_and_serves_a_segment_end_to_end() {
    let input = fixture();
    let input_path = input.to_string_lossy().into_owned();

    let probe = FfprobeMediaProbe::default()
        .probe(&input_path)
        .await
        .expect("ffprobe should succeed; is ffmpeg installed and on PATH?");
    assert!(!probe.video.is_empty(), "fixture must have a video track");
    assert!(probe.duration_ms > 0);

    let cache = tempfile::tempdir().unwrap();
    let session = SessionId(SESSION.to_owned());
    let manager = FfmpegTranscodeManager::new(cache.path());
    let started = manager
        .start(TranscodeSpec {
            session: session.clone(),
            input_path,
            seek_ms: None,
            audio_track: None,
            max_height: Some(180),
            max_bitrate: None,
            burn_subtitle_path: None,
        })
        .await
        .expect("ffmpeg transcode should start");

    let output_dir = PathBuf::from(&started.output_dir);
    let segment = wait_for_completed_segment(&output_dir.join("v0")).await;

    let source = HlsStreamSource::new();
    source.register(
        session.clone(),
        StreamRegistration {
            mode: DeliveryMode::Transcode,
            output_dir,
            direct_path: None,
            bandwidth: 1_000_000,
            subtitle: None,
        },
    );

    let tokens = HmacStreamTokens::new(SECRET);
    let token = tokens
        .create(&StreamClaims {
            session,
            user: UserId("smoke-user".to_owned()),
            version: VersionId("smoke".to_owned()),
            expires_at: Timestamp::from_second(Timestamp::now().as_second() + 3600).unwrap(),
            nonce: String::new(),
        })
        .expect("token mint should succeed")
        .0;

    let app = stream_router(StreamState::new(tokens, source));

    let (status, headers, body) =
        get(app.clone(), &format!("/stream/{token}/master.m3u8"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "application/vnd.apple.mpegurl"
    );
    assert!(body.starts_with(b"#EXTM3U"));
    assert!(String::from_utf8_lossy(&body).contains("v0/index.m3u8"));

    let (status, _, _) = get(app.clone(), &format!("/stream/{token}/v0/index.m3u8"), None).await;
    assert_eq!(status, StatusCode::OK);

    let segment_uri = format!("/stream/{token}/v0/{segment}");
    let (status, headers, body) = get(app.clone(), &segment_uri, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers.get(header::CONTENT_TYPE).unwrap(), "video/mp2t");
    assert!(!body.is_empty(), "served segment must have bytes");

    let (status, headers, _) = get(app, &segment_uri, Some("bytes=0-3")).await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert!(headers.contains_key(header::CONTENT_RANGE));
}

use std::path::PathBuf;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode, header};
use jiff::Timestamp;
use tower::ServiceExt;

use api::{StreamState, stream_router};
use domain::catalog::VersionId;
use domain::media::MediaProbe;
use domain::session::{
    DeliveryMode, SegmentContainer, SessionId, StreamClaims, StreamRegistration, StreamRegistry,
    StreamTokens, TranscodeManager, TranscodeSpec,
};
use domain::user::UserId;
use media::hls::HlsStreamSource;
use media::probe::FfprobeMediaProbe;
use media::stream_token::HmacStreamTokens;

const SECRET: &[u8] = b"shadowmask-smoke-secret";
const SESSION: &str = "smoke-session";

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../media/tests/fixtures/sample_real_bbb.mp4")
        .canonicalize()
        .expect("fixture sample_real_bbb.mp4 should exist")
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
async fn serves_a_jit_segment_end_to_end() {
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
    let engine = HlsStreamSource::new(cache.path());
    let started = engine
        .start(TranscodeSpec {
            session: session.clone(),
            input_path,
            duration_ms: probe.duration_ms,
            copy: true,
            container: SegmentContainer::MpegTs,
            seek_ms: None,
            audio_track: None,
            max_height: None,
            max_bitrate: None,
            burn_subtitle_path: None,
            soft_subtitle: None,
            downmix_stereo: false,
            source_hdr: None,
        })
        .await
        .expect("session start should succeed");

    let output_dir = PathBuf::from(&started.output_dir);
    engine.register(
        session.clone(),
        StreamRegistration {
            mode: DeliveryMode::Transcode,
            output_dir: output_dir.clone(),
            direct_path: None,
            bandwidth: 1_000_000,
            subtitle: None,
        },
    );

    let playlist = std::fs::read_to_string(output_dir.join("v0").join("index.m3u8")).unwrap();
    assert!(
        playlist.contains("#EXT-X-ENDLIST"),
        "playlist must be a finished VOD from the start"
    );
    let segment = playlist
        .lines()
        .rfind(|line| line.starts_with("seg_") && line.ends_with(".ts"))
        .expect("playlist must list at least one segment")
        .to_owned();

    let tokens = HmacStreamTokens::new(SECRET);
    let token = tokens
        .create(&StreamClaims {
            session,
            user: UserId("smoke-user".to_owned()),
            version: VersionId("smoke".to_owned()),
            expires_at: Timestamp::from_second(Timestamp::now().as_second() + 3600).unwrap(),
            nonce: String::new(),
        })
        .expect("token creation should succeed")
        .0;

    let app = stream_router(StreamState::new(tokens, engine));

    let (status, headers, body) =
        get(app.clone(), &format!("/stream/{token}/master.m3u8"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "application/vnd.apple.mpegurl"
    );
    assert!(body.starts_with(b"#EXTM3U"));
    assert!(String::from_utf8_lossy(&body).contains("v0/index.m3u8"));

    let (status, _, body) = get(app.clone(), &format!("/stream/{token}/v0/index.m3u8"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8_lossy(&body).contains("#EXT-X-ENDLIST"));

    let segment_uri = format!("/stream/{token}/v0/{segment}");
    let (status, headers, body) = get(app.clone(), &segment_uri, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers.get(header::CONTENT_TYPE).unwrap(), "video/mp2t");
    assert!(!body.is_empty(), "served segment must have bytes");

    let (status, headers, _) = get(app, &segment_uri, Some("bytes=0-3")).await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert!(headers.contains_key(header::CONTENT_RANGE));
}

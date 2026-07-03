use std::path::PathBuf;

use domain::error::ProbeError;
use domain::media::{MediaProbe, ProbeResult, SubtitleFormat};
use media::probe::FfprobeMediaProbe;

fn fixture(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

async fn probe(name: &str) -> ProbeResult {
    FfprobeMediaProbe::default()
        .probe(&fixture(name))
        .await
        .expect("ffprobe should succeed; is ffmpeg installed and on PATH?")
}

fn assert_duration_near(actual_ms: u64, expected_ms: u64) {
    let diff = actual_ms.abs_diff(expected_ms);
    assert!(
        diff <= 200,
        "duration {actual_ms}ms is not within 200ms of expected {expected_ms}ms"
    );
}

#[tokio::test]
async fn probes_generated_file_with_all_content() {
    let r = probe("sample_full.mkv").await;
    assert_duration_near(r.duration_ms, 2023);

    assert_eq!(r.video.len(), 1);
    let v = &r.video[0];
    assert_eq!(v.codec, "h264");
    assert_eq!(v.width, 160);
    assert_eq!(v.height, 120);
    assert!((v.frame_rate - 24.0).abs() < 0.01);

    assert_eq!(r.audio.len(), 1);
    let a = &r.audio[0];
    assert_eq!(a.codec, "aac");
    assert_eq!(a.channels, 1);
    assert_eq!(a.language.as_ref().map(|l| l.0.as_str()), Some("eng"));

    assert_eq!(r.subtitles.len(), 1);
    assert_eq!(r.subtitles[0].format, SubtitleFormat::Srt);

    assert_eq!(r.chapters.len(), 2);
    assert_eq!(r.chapters[0].title, "Intro");
    assert_eq!(r.chapters[0].start_ms, 0);
    assert_eq!(r.chapters[1].title, "Outro");
    assert_eq!(r.chapters[1].start_ms, 1000);
}

#[tokio::test]
async fn probes_fate_suite_file() {
    let r = probe("sample_fate_cinepak.avi").await;
    assert_duration_near(r.duration_ms, 30266);

    assert_eq!(r.video.len(), 1);
    assert_eq!(r.video[0].codec, "cinepak");
    assert_eq!(r.video[0].width, 100);
    assert_eq!(r.video[0].height, 75);

    assert_eq!(r.audio.len(), 1);
    assert_eq!(r.audio[0].codec, "pcm_u8");
    assert_eq!(r.audio[0].channels, 1);
}

#[tokio::test]
async fn probes_real_world_file() {
    let r = probe("sample_real_bbb.mp4").await;
    assert_duration_near(r.duration_ms, 10000);

    assert_eq!(r.video.len(), 1);
    let v = &r.video[0];
    assert_eq!(v.codec, "h264");
    assert_eq!(v.width, 640);
    assert_eq!(v.height, 360);
}

#[tokio::test]
async fn probe_reports_backend_error_for_missing_file() {
    let err = FfprobeMediaProbe::default()
        .probe("/nonexistent/shadowmask/file.mkv")
        .await
        .expect_err("probing a missing file must fail");
    assert!(matches!(err, ProbeError::Backend(_)));
}

#[tokio::test]
async fn probe_reports_backend_error_when_binary_missing() {
    let err = FfprobeMediaProbe::with_binary("shadowmask-no-such-ffprobe-binary")
        .probe(&fixture("sample_full.mkv"))
        .await
        .expect_err("spawning a missing binary must fail");
    assert!(matches!(err, ProbeError::Backend(_)));
}

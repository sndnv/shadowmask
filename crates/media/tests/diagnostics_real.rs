use std::path::PathBuf;
use std::time::Duration;

use domain::catalog::VersionId;
use domain::diagnostics::{
    AccelerationMode, BenchmarkOutcome, BenchmarkTarget, CapabilityProbe, HardwareTest,
    TranscodeBenchmark,
};
use media::diagnostics::{FfmpegCapabilityProbe, SegmentBenchmark};
use media::transcode::VideoEncoder;

#[tokio::test]
async fn the_installed_ffmpeg_benchmarks_a_real_file() {
    let scratch = tempfile::tempdir().unwrap();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample_full.mkv");
    let benchmark =
        SegmentBenchmark::new(scratch.path().join("bench"), VideoEncoder::Software, None);

    benchmark
        .start(BenchmarkTarget {
            version: VersionId("fixture".to_owned()),
            path: path.to_string_lossy().into_owned(),
            duration_ms: 2_023,
            codec: Some("h264".to_owned()),
            width: Some(160),
            height: Some(120),
            hdr: None,
            audio_track: Some(1),
        })
        .unwrap();
    let status = loop {
        let status = benchmark.status();
        if !status.running {
            break status;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    };

    assert_eq!(status.keyframes_error, None);
    assert_eq!(status.runs.len(), 1);
    assert_eq!(status.runs[0].outcome, BenchmarkOutcome::Ok, "{:?}", status.runs[0]);
}

#[tokio::test]
async fn the_installed_ffmpeg_reports_its_version_and_software_encoder() {
    let report = FfmpegCapabilityProbe::new(AccelerationMode::Off, "/dev/dri/renderD128", None)
        .probe()
        .await;

    assert_eq!(report.ffmpeg.error, None, "is ffmpeg installed and on PATH?");
    assert!(report.ffmpeg.version.is_some());
    assert!(report.ffmpeg.has_encoder("libx264"));
    assert!(report.host.threads >= 1);
    assert!(matches!(report.hardware.test, HardwareTest::Skipped { .. }));
}

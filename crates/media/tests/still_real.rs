use std::path::PathBuf;

use domain::error::StillError;
use domain::media::StillExtractor;
use media::still::FfmpegStillExtractor;

fn fixture(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

#[tokio::test]
async fn extracts_one_jpeg_frame_from_fixture() {
    let bytes = FfmpegStillExtractor::new()
        .extract(&fixture("sample_real_bbb.mp4"), 1_000)
        .await
        .expect("ffmpeg should produce a frame; is ffmpeg installed and on PATH?");

    assert_eq!(&bytes[..3], &[0xFF, 0xD8, 0xFF], "the frame must be a JPEG");
    assert_eq!(&bytes[bytes.len() - 2..], &[0xFF, 0xD9], "the JPEG must be written whole");
}

#[tokio::test]
async fn extract_reports_backend_error_for_missing_input() {
    let err = FfmpegStillExtractor::new()
        .extract("/nonexistent/shadowmask/movie.mkv", 0)
        .await
        .expect_err("a missing input must fail");
    assert!(matches!(err, StillError::Backend(_)));
}

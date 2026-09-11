use std::path::PathBuf;

use domain::catalog::VersionId;
use domain::error::TrickplayError;
use domain::media::TrickplayGenerator;
use media::trickplay::{FfmpegTrickplayGenerator, TrickplayConfig};

fn fixture(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn small_config() -> TrickplayConfig {
    TrickplayConfig { interval_ms: 2_000, columns: 3, rows: 3, tile_width: 160, tile_height: 90 }
}

#[tokio::test]
async fn generates_sheets_from_fixture() {
    let cache = tempfile::tempdir().expect("tempdir");
    let generator = FfmpegTrickplayGenerator::new(cache.path()).with_config(small_config());

    let asset = generator
        .generate(&fixture("sample_real_bbb.mp4"), &VersionId("bbb".to_owned()), 10_000)
        .await
        .expect("ffmpeg should produce trickplay sheets; is ffmpeg installed and on PATH?");

    assert_eq!(asset.tile_width, 160);
    assert_eq!(asset.tile_height, 90);
    assert_eq!(asset.sheet_paths.len(), 1);
    assert!(
        PathBuf::from(&asset.sheet_paths[0]).is_file(),
        "expected sprite sheet at {}",
        asset.sheet_paths[0]
    );
}

#[tokio::test]
async fn generate_reports_backend_error_for_missing_input() {
    let cache = tempfile::tempdir().expect("tempdir");
    let generator = FfmpegTrickplayGenerator::new(cache.path()).with_config(small_config());

    let err = generator
        .generate("/nonexistent/shadowmask/movie.mkv", &VersionId("v1".to_owned()), 10_000)
        .await
        .expect_err("a missing input must fail");
    assert!(matches!(err, TrickplayError::Backend(_)));
}

#[tokio::test]
async fn generate_reports_backend_error_when_binary_missing() {
    let cache = tempfile::tempdir().expect("tempdir");
    let generator =
        FfmpegTrickplayGenerator::new(cache.path()).with_binary("shadowmask-no-such-ffmpeg-binary");

    let err = generator
        .generate(&fixture("sample_real_bbb.mp4"), &VersionId("v1".to_owned()), 10_000)
        .await
        .expect_err("spawning a missing binary must fail");
    assert!(matches!(err, TrickplayError::Backend(_)));
}

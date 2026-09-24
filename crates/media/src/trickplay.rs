use std::path::{Path, PathBuf};

use tokio::process::Command;

use domain::catalog::VersionId;
use domain::error::{CacheError, TrickplayError};
use domain::media::{
    DerivedAssetDir, DerivedAssetFile, DerivedAssetStore, TrickplayAsset, TrickplayGenerator,
};

const DEFAULT_BINARY: &str = "ffmpeg";

pub struct TrickplayConfig {
    pub interval_ms: u64,
    pub columns: u32,
    pub rows: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub threads: usize,
    pub keyframes_only: bool,
}

impl Default for TrickplayConfig {
    fn default() -> Self {
        Self {
            interval_ms: 10_000,
            columns: 10,
            rows: 10,
            tile_width: 320,
            tile_height: 180,
            threads: 2,
            keyframes_only: true,
        }
    }
}

pub struct FfmpegTrickplayGenerator {
    binary: String,
    cache_root: PathBuf,
    config: TrickplayConfig,
}

impl FfmpegTrickplayGenerator {
    pub fn new(cache_root: impl Into<PathBuf>) -> Self {
        Self {
            binary: DEFAULT_BINARY.to_owned(),
            cache_root: cache_root.into(),
            config: TrickplayConfig::default(),
        }
    }

    pub fn with_binary(mut self, binary: impl Into<String>) -> Self {
        self.binary = binary.into();
        self
    }

    pub fn with_config(mut self, config: TrickplayConfig) -> Self {
        self.config = config;
        self
    }
}

impl TrickplayGenerator for FfmpegTrickplayGenerator {
    async fn generate(
        &self,
        input_path: &str,
        version: &VersionId,
        duration_ms: u64,
    ) -> Result<TrickplayAsset, TrickplayError> {
        let output_dir = self.cache_root.join(&version.0);
        std::fs::create_dir_all(&output_dir).map_err(|e| TrickplayError::Backend(e.to_string()))?;
        let (args, asset) =
            plan_trickplay(input_path, duration_ms, &self.config, version, &output_dir);
        let output = Command::new(&self.binary)
            .args(&args)
            .output()
            .await
            .map_err(|e| TrickplayError::Backend(e.to_string()))?;
        if !output.status.success() {
            return Err(TrickplayError::Backend(
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }
        Ok(asset)
    }
}

impl DerivedAssetStore for FfmpegTrickplayGenerator {
    fn label(&self) -> &'static str {
        "trickplay"
    }

    async fn list_dirs(&self) -> Result<Vec<DerivedAssetDir>, CacheError> {
        crate::derived_assets::list_dirs(&self.cache_root).await
    }

    async fn remove_dir(&self, owner: &str) -> Result<(), CacheError> {
        crate::derived_assets::remove_dir(&self.cache_root, owner).await
    }

    async fn list_files(&self, owner: &str) -> Result<Vec<DerivedAssetFile>, CacheError> {
        crate::derived_assets::list_files(&self.cache_root, owner).await
    }

    async fn remove_file(&self, path: &str) -> Result<(), CacheError> {
        crate::derived_assets::remove_file(&self.cache_root, path).await
    }
}

fn plan_trickplay(
    input_path: &str,
    duration_ms: u64,
    config: &TrickplayConfig,
    version: &VersionId,
    output_dir: &Path,
) -> (Vec<String>, TrickplayAsset) {
    let per_sheet = (u64::from(config.columns) * u64::from(config.rows)).max(1);
    let tiles = duration_ms.div_ceil(config.interval_ms.max(1));
    let sheets = tiles.div_ceil(per_sheet);
    let dir = output_dir.to_string_lossy();
    let sheet_paths = (1..=sheets).map(|i| format!("{dir}/sheet-{i:03}.jpg")).collect();
    let filter = format!(
        "fps=1/{},scale={}:{},tile={}x{}",
        config.interval_ms as f64 / 1000.0,
        config.tile_width,
        config.tile_height,
        config.columns,
        config.rows
    );
    let threads = config.threads.max(1).to_string();
    let mut args = vec!["-v".to_owned(), "error".to_owned(), "-nostdin".to_owned()];
    if config.keyframes_only {
        args.extend(["-skip_frame".to_owned(), "nokey".to_owned()]);
    }
    args.extend([
        "-threads".to_owned(),
        threads.clone(),
        "-filter_threads".to_owned(),
        threads,
        "-i".to_owned(),
        input_path.to_owned(),
        "-vf".to_owned(),
        filter,
        "-q:v".to_owned(),
        "4".to_owned(),
        format!("{dir}/sheet-%03d.jpg"),
    ]);
    let asset = TrickplayAsset {
        version: version.clone(),
        interval_ms: config.interval_ms,
        columns: config.columns,
        rows: config.rows,
        tile_width: config.tile_width,
        tile_height: config.tile_height,
        sheet_paths,
    };
    (args, asset)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version() -> VersionId {
        VersionId("v1".to_owned())
    }

    fn plan(duration_ms: u64, config: TrickplayConfig) -> (Vec<String>, TrickplayAsset) {
        plan_trickplay("/media/movie.mkv", duration_ms, &config, &version(), Path::new("/cache/v1"))
    }

    #[test]
    fn exact_multiple_duration_fills_whole_sheets() {
        let config = TrickplayConfig {
            interval_ms: 10_000,
            columns: 2,
            rows: 1,
            ..TrickplayConfig::default()
        };
        let (_args, asset) = plan(40_000, config);
        assert_eq!(asset.sheet_paths.len(), 2);
        assert_eq!(asset.sheet_paths[0], "/cache/v1/sheet-001.jpg");
        assert_eq!(asset.sheet_paths[1], "/cache/v1/sheet-002.jpg");
    }

    #[test]
    fn remainder_duration_rounds_up_tiles_and_sheets() {
        let config = TrickplayConfig {
            interval_ms: 10_000,
            columns: 2,
            rows: 1,
            ..TrickplayConfig::default()
        };
        let (_args, asset) = plan(50_000, config);
        assert_eq!(asset.sheet_paths.len(), 3);
    }

    #[test]
    fn zero_duration_produces_no_sheets() {
        let (_args, asset) = plan(0, TrickplayConfig::default());
        assert!(asset.sheet_paths.is_empty());
    }

    #[test]
    fn builds_expected_filter_and_output_args() {
        let config = TrickplayConfig {
            interval_ms: 5_000,
            columns: 4,
            rows: 4,
            tile_width: 320,
            tile_height: 180,
            threads: 3,
            keyframes_only: false,
        };
        let (args, asset) = plan(5_000, config);
        assert_eq!(
            args,
            vec![
                "-v".to_owned(),
                "error".to_owned(),
                "-nostdin".to_owned(),
                "-threads".to_owned(),
                "3".to_owned(),
                "-filter_threads".to_owned(),
                "3".to_owned(),
                "-i".to_owned(),
                "/media/movie.mkv".to_owned(),
                "-vf".to_owned(),
                "fps=1/5,scale=320:180,tile=4x4".to_owned(),
                "-q:v".to_owned(),
                "4".to_owned(),
                "/cache/v1/sheet-%03d.jpg".to_owned(),
            ]
        );
        assert_eq!(asset.interval_ms, 5_000);
        assert_eq!(asset.columns, 4);
        assert_eq!(asset.rows, 4);
        assert_eq!(asset.tile_width, 320);
        assert_eq!(asset.tile_height, 180);
        assert_eq!(asset.version, version());
    }

    #[test]
    fn keyframe_only_decoding_is_on_by_default_and_can_be_turned_off() {
        let (with, _asset) = plan(10_000, TrickplayConfig::default());
        let without =
            plan(10_000, TrickplayConfig { keyframes_only: false, ..Default::default() }).0;

        assert!(with.windows(2).any(|pair| pair == ["-skip_frame", "nokey"]));
        assert!(!without.iter().any(|arg| arg == "-skip_frame"));
        assert!(
            with.iter().position(|arg| arg == "-skip_frame")
                < with.iter().position(|arg| arg == "-i")
        );
    }

    #[test]
    fn zero_threads_is_floored_to_one() {
        let config = TrickplayConfig { threads: 0, ..TrickplayConfig::default() };
        let (args, _asset) = plan(10_000, config);
        let threads = args.windows(2).find(|pair| pair[0] == "-threads").expect("threads arg");
        assert_eq!(threads[1], "1");
    }

    #[tokio::test]
    async fn generate_fails_when_output_dir_cannot_be_created() {
        let file = tempfile::NamedTempFile::new().expect("tempfile");
        let generator = FfmpegTrickplayGenerator::new(file.path());
        let err = generator.generate("/media/movie.mkv", &version(), 10_000).await.unwrap_err();
        assert!(matches!(err, TrickplayError::Backend(_)));
    }

    #[tokio::test]
    async fn the_sweep_sees_one_dir_per_version_and_can_remove_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let generator = FfmpegTrickplayGenerator::new(dir.path());
        assert_eq!(DerivedAssetStore::label(&generator), "trickplay");
        let sheets = dir.path().join("v1");
        tokio::fs::create_dir(&sheets).await.expect("create");
        tokio::fs::write(sheets.join("0.jpg"), b"jpg").await.expect("write");

        let dirs = generator.list_dirs().await.expect("list");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].owner, "v1");

        let files = generator.list_files("v1").await.expect("list files");
        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("0.jpg"));
        generator.remove_file(&files[0].path).await.expect("remove file");
        assert!(generator.list_files("v1").await.expect("list files").is_empty());

        generator.remove_dir("v1").await.expect("remove");
        assert!(generator.list_dirs().await.expect("list").is_empty());
    }
}

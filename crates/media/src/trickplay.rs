use std::path::{Path, PathBuf};

use tokio::process::Command;

use domain::catalog::VersionId;
use domain::error::TrickplayError;
use domain::media::{TrickplayAsset, TrickplayGenerator};

const DEFAULT_BINARY: &str = "ffmpeg";

pub struct TrickplayConfig {
    pub interval_ms: u64,
    pub columns: u32,
    pub rows: u32,
    pub tile_width: u32,
    pub tile_height: u32,
}

impl Default for TrickplayConfig {
    fn default() -> Self {
        Self {
            interval_ms: 10_000,
            columns: 10,
            rows: 10,
            tile_width: 320,
            tile_height: 180,
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
        let (args, asset) = plan_trickplay(duration_ms, &self.config, version, &output_dir);
        let output = Command::new(&self.binary)
            .args(["-v", "error", "-i"])
            .arg(input_path)
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

fn plan_trickplay(
    duration_ms: u64,
    config: &TrickplayConfig,
    version: &VersionId,
    output_dir: &Path,
) -> (Vec<String>, TrickplayAsset) {
    let per_sheet = (u64::from(config.columns) * u64::from(config.rows)).max(1);
    let tiles = duration_ms.div_ceil(config.interval_ms.max(1));
    let sheets = tiles.div_ceil(per_sheet);
    let dir = output_dir.to_string_lossy();
    let sheet_paths = (1..=sheets)
        .map(|i| format!("{dir}/sheet-{i:03}.jpg"))
        .collect();
    let filter = format!(
        "fps=1/{},scale={}:{},tile={}x{}",
        config.interval_ms as f64 / 1000.0,
        config.tile_width,
        config.tile_height,
        config.columns,
        config.rows
    );
    let args = vec![
        "-vf".to_owned(),
        filter,
        "-q:v".to_owned(),
        "4".to_owned(),
        format!("{dir}/sheet-%03d.jpg"),
    ];
    let asset = TrickplayAsset {
        version: version.clone(),
        interval_ms: config.interval_ms,
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
        plan_trickplay(duration_ms, &config, &version(), Path::new("/cache/v1"))
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
        };
        let (args, asset) = plan(5_000, config);
        assert_eq!(
            args,
            vec![
                "-vf".to_owned(),
                "fps=1/5,scale=320:180,tile=4x4".to_owned(),
                "-q:v".to_owned(),
                "4".to_owned(),
                "/cache/v1/sheet-%03d.jpg".to_owned(),
            ]
        );
        assert_eq!(asset.interval_ms, 5_000);
        assert_eq!(asset.tile_width, 320);
        assert_eq!(asset.tile_height, 180);
        assert_eq!(asset.version, version());
    }

    #[tokio::test]
    async fn generate_fails_when_output_dir_cannot_be_created() {
        let file = tempfile::NamedTempFile::new().expect("tempfile");
        let generator = FfmpegTrickplayGenerator::new(file.path());
        let err = generator
            .generate("/media/movie.mkv", &version(), 10_000)
            .await
            .unwrap_err();
        assert!(matches!(err, TrickplayError::Backend(_)));
    }
}

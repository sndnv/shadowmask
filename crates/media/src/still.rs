use tokio::process::Command;

use domain::error::StillError;
use domain::media::StillExtractor;

const DEFAULT_BINARY: &str = "ffmpeg";
const FILTER: &str =
    "scale=w='trunc(min(960,iw*sar)/2)*2':h='trunc(ow/dar/2)*2',setsar=1,thumbnail";

pub struct FfmpegStillExtractor {
    binary: String,
}

impl FfmpegStillExtractor {
    pub fn new() -> Self {
        Self { binary: DEFAULT_BINARY.to_owned() }
    }

    pub fn with_binary(mut self, binary: impl Into<String>) -> Self {
        self.binary = binary.into();
        self
    }
}

impl Default for FfmpegStillExtractor {
    fn default() -> Self {
        Self::new()
    }
}

fn still_args(input_path: &str, offset_ms: u64) -> Vec<String> {
    let offset = format!("{}.{:03}", offset_ms / 1000, offset_ms % 1000);
    [
        "-v",
        "error",
        "-nostdin",
        "-ss",
        &offset,
        "-i",
        input_path,
        "-an",
        "-sn",
        "-dn",
        "-vf",
        FILTER,
        "-frames:v",
        "1",
        "-q:v",
        "2",
        "-f",
        "image2pipe",
        "-c:v",
        "mjpeg",
        "pipe:1",
    ]
    .map(ToOwned::to_owned)
    .to_vec()
}

impl StillExtractor for FfmpegStillExtractor {
    async fn extract(&self, input_path: &str, offset_ms: u64) -> Result<Vec<u8>, StillError> {
        let output = Command::new(&self.binary)
            .args(still_args(input_path, offset_ms))
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|e| StillError::Backend(e.to_string()))?;
        if !output.status.success() {
            return Err(StillError::Backend(String::from_utf8_lossy(&output.stderr).into_owned()));
        }
        if output.stdout.is_empty() {
            return Err(StillError::Backend(format!("ffmpeg wrote no frame for [{input_path}]")));
        }
        Ok(output.stdout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_frame_is_sought_before_the_input_and_written_to_stdout_as_one_jpeg() {
        assert_eq!(
            still_args("/media/Show - S01E02.mkv", 61_500),
            [
                "-v",
                "error",
                "-nostdin",
                "-ss",
                "61.500",
                "-i",
                "/media/Show - S01E02.mkv",
                "-an",
                "-sn",
                "-dn",
                "-vf",
                "scale=w='trunc(min(960,iw*sar)/2)*2':h='trunc(ow/dar/2)*2',setsar=1,thumbnail",
                "-frames:v",
                "1",
                "-q:v",
                "2",
                "-f",
                "image2pipe",
                "-c:v",
                "mjpeg",
                "pipe:1",
            ]
        );
    }

    #[test]
    fn an_offset_of_zero_seeks_to_the_start() {
        let args = still_args("/media/movie.mkv", 0);
        assert!(args.windows(2).any(|pair| pair == ["-ss", "0.000"]));
    }

    #[tokio::test]
    async fn a_binary_that_will_not_start_is_a_backend_error() {
        let extractor = FfmpegStillExtractor::new().with_binary("shadowmask-no-such-ffmpeg-binary");
        let err = extractor.extract("/media/movie.mkv", 0).await.unwrap_err();
        assert!(matches!(err, StillError::Backend(_)));
    }

    #[tokio::test]
    async fn a_failing_run_is_a_backend_error() {
        let err = FfmpegStillExtractor::default()
            .with_binary("false")
            .extract("/media/movie.mkv", 0)
            .await
            .unwrap_err();
        assert!(matches!(err, StillError::Backend(_)));
    }

    #[tokio::test]
    async fn a_run_that_writes_no_frame_is_a_backend_error() {
        let err = FfmpegStillExtractor::new()
            .with_binary("true")
            .extract("/media/movie.mkv", 0)
            .await
            .unwrap_err();
        assert!(matches!(&err, StillError::Backend(detail) if detail.contains("no frame")));
    }
}

use std::path::Path;

use domain::error::UpscaleError;
use domain::media::{MediaProbe, UpscaleOutput, UpscaleProvider, UpscaleSpec};

use crate::probe::FfprobeMediaProbe;
use crate::transcode::{TokioProcessSpawner, VideoEncoder};
use domain::process::ProcessSpawner;

const DEFAULT_BINARY: &str = "ffmpeg";

#[derive(Debug, Clone)]
pub struct FfmpegUpscaler<S = TokioProcessSpawner, P = FfprobeMediaProbe> {
    spawner: S,
    probe: P,
    binary: String,
    encoder: VideoEncoder,
}

impl Default for FfmpegUpscaler {
    fn default() -> Self {
        Self::with_parts(TokioProcessSpawner, FfprobeMediaProbe::default())
    }
}

impl FfmpegUpscaler {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<S, P> FfmpegUpscaler<S, P> {
    pub fn with_parts(spawner: S, probe: P) -> Self {
        Self { spawner, probe, binary: DEFAULT_BINARY.to_owned(), encoder: VideoEncoder::Software }
    }

    pub fn with_encoder(mut self, encoder: VideoEncoder) -> Self {
        self.encoder = encoder;
        self
    }
}

fn target_width(source_width: u32, source_height: u32, target_height: u32) -> u32 {
    let width = (source_width as u64 * target_height as u64) / source_height as u64;
    ((width & !1) as u32).max(2)
}

pub(crate) fn build_upscale_args(
    source: &str,
    output: &str,
    width: u32,
    height: u32,
    encoder: &VideoEncoder,
) -> Vec<String> {
    let mut args = vec!["-nostdin".to_owned(), "-y".to_owned()];
    if let VideoEncoder::Vaapi { device } = encoder {
        args.push("-vaapi_device".to_owned());
        args.push(device.clone());
    }
    args.push("-i".to_owned());
    args.push(source.to_owned());
    args.push("-map".to_owned());
    args.push("0:v:0".to_owned());
    args.push("-map".to_owned());
    args.push("0:a?".to_owned());
    match encoder {
        VideoEncoder::Vaapi { .. } => {
            args.push("-c:v".to_owned());
            args.push("h264_vaapi".to_owned());
            args.push("-vf".to_owned());
            args.push(format!("scale={width}:{height}:flags=lanczos,format=nv12,hwupload"));
        }
        VideoEncoder::Software => {
            args.push("-c:v".to_owned());
            args.push("libx264".to_owned());
            args.push("-preset".to_owned());
            args.push("veryfast".to_owned());
            args.push("-pix_fmt".to_owned());
            args.push("yuv420p".to_owned());
            args.push("-vf".to_owned());
            args.push(format!("scale={width}:{height}:flags=lanczos"));
        }
    }
    args.push("-c:a".to_owned());
    args.push("copy".to_owned());
    args.push(output.to_owned());
    args
}

async fn ensure_writable(output: &Path) -> Result<(), UpscaleError> {
    let probe = output.with_extension("shadowmask-writetest");
    tokio::fs::write(&probe, b"")
        .await
        .map_err(|e| UpscaleError::Precondition(format!("target directory not writable: {e}")))?;
    let _ = tokio::fs::remove_file(&probe).await;
    Ok(())
}

impl<S: ProcessSpawner, P: MediaProbe + Send + Sync> UpscaleProvider for FfmpegUpscaler<S, P> {
    async fn upscale(&self, request: &UpscaleSpec) -> Result<UpscaleOutput, UpscaleError> {
        let probe = self
            .probe
            .probe(&request.source_path)
            .await
            .map_err(|e| UpscaleError::Backend(e.to_string()))?;
        let Some(video) = probe.video.first() else {
            return Err(UpscaleError::Unsupported("source has no video track".to_owned()));
        };
        if video.height >= request.target_height {
            return Err(UpscaleError::Unsupported(format!(
                "source height {} already meets target {}",
                video.height, request.target_height
            )));
        }
        let width = target_width(video.width, video.height, request.target_height);

        let output = Path::new(&request.output_path);
        if tokio::fs::try_exists(output).await.unwrap_or(false) {
            return Err(UpscaleError::Precondition(format!(
                "output path already exists: {}",
                request.output_path
            )));
        }
        ensure_writable(output).await?;

        let mut attempts = vec![build_upscale_args(
            &request.source_path,
            &request.output_path,
            width,
            request.target_height,
            &self.encoder,
        )];
        if self.encoder.is_hardware() {
            attempts.push(build_upscale_args(
                &request.source_path,
                &request.output_path,
                width,
                request.target_height,
                &VideoEncoder::Software,
            ));
        }
        let mut produced = false;
        let mut last_error = "ffmpeg exited with a failure status".to_owned();
        for args in &attempts {
            match self.spawner.run(&self.binary, args).await {
                Ok(true) => {
                    produced = true;
                    break;
                }
                Ok(false) => {
                    let _ = tokio::fs::remove_file(output).await;
                }
                Err(e) => {
                    let _ = tokio::fs::remove_file(output).await;
                    last_error = e.to_string();
                }
            }
        }
        if !produced {
            return Err(UpscaleError::Backend(last_error));
        }

        let size_bytes = tokio::fs::metadata(output)
            .await
            .map_err(|e| UpscaleError::Backend(e.to_string()))?
            .len();
        Ok(UpscaleOutput { path: request.output_path.clone(), size_bytes })
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use domain::error::ProbeError;
    use domain::media::{ProbeResult, VideoTrack};

    use super::*;

    fn video(width: u32, height: u32) -> VideoTrack {
        VideoTrack {
            index: 0,
            codec: "h264".to_owned(),
            width,
            height,
            bit_depth: 8,
            hdr: None,
            frame_rate: 24.0,
            bitrate: None,
        }
    }

    fn probe_of(video: Vec<VideoTrack>) -> ProbeResult {
        ProbeResult {
            duration_ms: 1000,
            video,
            audio: Vec::new(),
            subtitles: Vec::new(),
            chapters: Vec::new(),
        }
    }

    #[derive(Clone)]
    struct MockProbe {
        result: Result<ProbeResult, ()>,
    }

    impl MediaProbe for MockProbe {
        async fn probe(&self, _path: &str) -> Result<ProbeResult, ProbeError> {
            self.result.clone().map_err(|_| ProbeError::Backend("probe failed".to_owned()))
        }
    }

    #[derive(Clone)]
    struct MockSpawner {
        succeed: bool,
        error: bool,
        write: bool,
    }

    impl ProcessSpawner for MockSpawner {
        async fn run(&self, _program: &str, args: &[String]) -> io::Result<bool> {
            if self.error {
                return Err(io::Error::other("spawn failed"));
            }
            if self.write {
                let output = args.last().expect("output arg");
                tokio::fs::write(output, b"upscaled").await.expect("mock writes output");
            }
            Ok(self.succeed)
        }
    }

    #[derive(Clone, Default)]
    struct FailFirstSpawner {
        calls: Arc<AtomicUsize>,
    }

    impl ProcessSpawner for FailFirstSpawner {
        async fn run(&self, _program: &str, args: &[String]) -> io::Result<bool> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            if call == 0 {
                return Ok(false);
            }
            let output = args.last().expect("output arg");
            tokio::fs::write(output, b"upscaled").await.expect("mock writes output");
            Ok(true)
        }
    }

    fn upscaler(probe: MockProbe, spawner: MockSpawner) -> FfmpegUpscaler<MockSpawner, MockProbe> {
        FfmpegUpscaler::with_parts(spawner, probe)
    }

    #[test]
    fn target_width_is_even_and_preserves_aspect() {
        assert_eq!(target_width(640, 480, 720), 960);
        assert_eq!(target_width(1280, 534, 1080), 2588);
        assert!(target_width(3, 2, 3) >= 2);
    }

    #[test]
    fn build_upscale_args_carries_lanczos_scale() {
        let args = build_upscale_args("in.mkv", "out.mp4", 960, 720, &VideoEncoder::Software);
        assert!(args.contains(&"scale=960:720:flags=lanczos".to_owned()));
        assert!(args.contains(&"libx264".to_owned()));
        assert_eq!(args.last().unwrap(), "out.mp4");
    }

    #[test]
    fn build_upscale_args_vaapi_uses_hardware_encoder_and_upload() {
        let args = build_upscale_args(
            "in.mkv",
            "out.mp4",
            960,
            720,
            &VideoEncoder::Vaapi { device: "/dev/dri/renderD128".to_owned() },
        );
        assert!(args.contains(&"-vaapi_device".to_owned()));
        assert!(args.contains(&"/dev/dri/renderD128".to_owned()));
        assert!(args.contains(&"h264_vaapi".to_owned()));
        assert!(args.contains(&"scale=960:720:flags=lanczos,format=nv12,hwupload".to_owned()));
        assert!(!args.contains(&"libx264".to_owned()));
        assert_eq!(args.last().unwrap(), "out.mp4");
    }

    #[tokio::test]
    async fn hardware_upscale_failure_falls_back_to_software() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("out.mp4").to_string_lossy().into_owned();
        let spawner = FailFirstSpawner::default();
        let up = FfmpegUpscaler::with_parts(
            spawner.clone(),
            MockProbe { result: Ok(probe_of(vec![video(640, 480)])) },
        )
        .with_encoder(VideoEncoder::Vaapi { device: "/dev/dri/renderD128".to_owned() });
        let out = up
            .upscale(&UpscaleSpec {
                source_path: "src.mkv".to_owned(),
                target_height: 720,
                output_path: output.clone(),
            })
            .await
            .expect("upscale via software fallback");
        assert_eq!(out.path, output);
        assert_eq!(spawner.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn upscales_a_lower_res_source() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("out.mp4").to_string_lossy().into_owned();
        let up = upscaler(
            MockProbe { result: Ok(probe_of(vec![video(640, 480)])) },
            MockSpawner { succeed: true, error: false, write: true },
        );
        let out = up
            .upscale(&UpscaleSpec {
                source_path: "src.mkv".to_owned(),
                target_height: 720,
                output_path: output.clone(),
            })
            .await
            .expect("upscale");
        assert_eq!(out.path, output);
        assert!(out.size_bytes > 0);
    }

    #[tokio::test]
    async fn rejects_source_without_video() {
        let up = upscaler(
            MockProbe { result: Ok(probe_of(Vec::new())) },
            MockSpawner { succeed: true, error: false, write: true },
        );
        let err = up
            .upscale(&UpscaleSpec {
                source_path: "src.mkv".to_owned(),
                target_height: 720,
                output_path: "/tmp/out.mp4".to_owned(),
            })
            .await
            .expect_err("no video");
        assert!(matches!(err, UpscaleError::Unsupported(_)));
    }

    #[tokio::test]
    async fn rejects_source_already_at_target() {
        let up = upscaler(
            MockProbe { result: Ok(probe_of(vec![video(1920, 1080)])) },
            MockSpawner { succeed: true, error: false, write: true },
        );
        let err = up
            .upscale(&UpscaleSpec {
                source_path: "src.mkv".to_owned(),
                target_height: 1080,
                output_path: "/tmp/out.mp4".to_owned(),
            })
            .await
            .expect_err("already at target");
        assert!(matches!(err, UpscaleError::Unsupported(_)));
    }

    #[tokio::test]
    async fn rejects_existing_output() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("out.mp4");
        std::fs::write(&output, b"existing").unwrap();
        let up = upscaler(
            MockProbe { result: Ok(probe_of(vec![video(640, 480)])) },
            MockSpawner { succeed: true, error: false, write: true },
        );
        let err = up
            .upscale(&UpscaleSpec {
                source_path: "src.mkv".to_owned(),
                target_height: 720,
                output_path: output.to_string_lossy().into_owned(),
            })
            .await
            .expect_err("existing output");
        assert!(matches!(err, UpscaleError::Precondition(_)));
    }

    #[tokio::test]
    async fn rejects_unwritable_target() {
        let dir = tempfile::tempdir().unwrap();
        let output =
            dir.path().join("missing-subdir").join("out.mp4").to_string_lossy().into_owned();
        let up = upscaler(
            MockProbe { result: Ok(probe_of(vec![video(640, 480)])) },
            MockSpawner { succeed: true, error: false, write: true },
        );
        let err = up
            .upscale(&UpscaleSpec {
                source_path: "src.mkv".to_owned(),
                target_height: 720,
                output_path: output,
            })
            .await
            .expect_err("unwritable target");
        assert!(matches!(err, UpscaleError::Precondition(_)));
    }

    #[tokio::test]
    async fn probe_failure_is_backend() {
        let up = upscaler(
            MockProbe { result: Err(()) },
            MockSpawner { succeed: true, error: false, write: true },
        );
        let err = up
            .upscale(&UpscaleSpec {
                source_path: "src.mkv".to_owned(),
                target_height: 720,
                output_path: "/tmp/out.mp4".to_owned(),
            })
            .await
            .expect_err("probe failed");
        assert!(matches!(err, UpscaleError::Backend(_)));
    }

    #[tokio::test]
    async fn ffmpeg_failure_is_backend() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("out.mp4").to_string_lossy().into_owned();
        let up = upscaler(
            MockProbe { result: Ok(probe_of(vec![video(640, 480)])) },
            MockSpawner { succeed: false, error: false, write: false },
        );
        let err = up
            .upscale(&UpscaleSpec {
                source_path: "src.mkv".to_owned(),
                target_height: 720,
                output_path: output,
            })
            .await
            .expect_err("ffmpeg failed");
        assert!(matches!(err, UpscaleError::Backend(_)));
    }

    #[tokio::test]
    async fn spawn_error_is_backend() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("out.mp4").to_string_lossy().into_owned();
        let up = upscaler(
            MockProbe { result: Ok(probe_of(vec![video(640, 480)])) },
            MockSpawner { succeed: false, error: true, write: false },
        );
        let err = up
            .upscale(&UpscaleSpec {
                source_path: "src.mkv".to_owned(),
                target_height: 720,
                output_path: output,
            })
            .await
            .expect_err("spawn error");
        assert!(matches!(err, UpscaleError::Backend(_)));
    }

    #[tokio::test]
    async fn missing_output_after_run_is_backend() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("out.mp4").to_string_lossy().into_owned();
        let up = upscaler(
            MockProbe { result: Ok(probe_of(vec![video(640, 480)])) },
            MockSpawner { succeed: true, error: false, write: false },
        );
        let err = up
            .upscale(&UpscaleSpec {
                source_path: "src.mkv".to_owned(),
                target_height: 720,
                output_path: output,
            })
            .await
            .expect_err("no output file");
        assert!(matches!(err, UpscaleError::Backend(_)));
    }
}

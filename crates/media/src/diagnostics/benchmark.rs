use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use domain::diagnostics::{
    BenchmarkOutcome, BenchmarkRun, BenchmarkStatus, BenchmarkTarget, TranscodeBenchmark,
};
use domain::error::BenchmarkError;
use domain::process::ProcessSpawner;
use domain::session::{
    Segment, SegmentContainer, SegmentPlan, SessionId, StreamGeneration, TranscodeSpec,
    plan_fixed_segments,
};
use jiff::Timestamp;

use crate::transcode::{
    RunWindow, TARGET_MS, TokioProcessSpawner, VideoEncoder, build_run_args, film_start,
};

const BINARY: &str = "ffmpeg";
const RUN_CAP: Duration = Duration::from_secs(60);
const RUN_SEGMENTS: u64 = 3;
const QUARTERS: [u64; 3] = [1, 2, 3];

pub struct SegmentBenchmark<S = TokioProcessSpawner> {
    inner: Arc<Inner<S>>,
}

struct Inner<S> {
    spawner: S,
    scratch: PathBuf,
    encoder: VideoEncoder,
    max_height: Option<u32>,
    cap: Duration,
    status: Mutex<BenchmarkStatus>,
}

impl SegmentBenchmark {
    pub fn new(
        scratch: impl Into<PathBuf>,
        encoder: VideoEncoder,
        max_height: Option<u32>,
    ) -> Self {
        Self::with_parts(TokioProcessSpawner, scratch, encoder, max_height, RUN_CAP)
    }
}

impl<S> SegmentBenchmark<S> {
    pub fn with_parts(
        spawner: S,
        scratch: impl Into<PathBuf>,
        encoder: VideoEncoder,
        max_height: Option<u32>,
        cap: Duration,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                spawner,
                scratch: scratch.into(),
                encoder,
                max_height,
                cap,
                status: Mutex::default(),
            }),
        }
    }
}

impl<S> Clone for SegmentBenchmark<S> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<S: ProcessSpawner + 'static> TranscodeBenchmark for SegmentBenchmark<S> {
    fn status(&self) -> BenchmarkStatus {
        self.inner.status.lock().unwrap().clone()
    }

    fn start(&self, target: BenchmarkTarget) -> Result<(), BenchmarkError> {
        {
            let mut status = self.inner.status.lock().unwrap();
            if status.running {
                return Err(BenchmarkError::Running);
            }
            *status = BenchmarkStatus {
                running: true,
                target: Some(target.clone()),
                started_at: Some(Timestamp::now()),
                ..BenchmarkStatus::default()
            };
        }
        tokio::spawn(Arc::clone(&self.inner).run(target));
        Ok(())
    }
}

impl<S: ProcessSpawner> Inner<S> {
    fn update(&self, change: impl FnOnce(&mut BenchmarkStatus)) {
        change(&mut self.status.lock().unwrap());
    }

    fn encoders(&self) -> Vec<VideoEncoder> {
        if self.encoder.is_hardware() {
            vec![self.encoder.clone(), VideoEncoder::Software]
        } else {
            vec![VideoEncoder::Software]
        }
    }

    async fn run(self: Arc<Self>, target: BenchmarkTarget) {
        let _running = Running(&self.status);
        let picked = picked(&plan_fixed_segments(target.duration_ms, TARGET_MS), &target);
        let encoders = self.encoders();
        self.update(|status| status.total = picked.len() * encoders.len());
        let spec = spec_for(&target, self.max_height);
        let start_us = film_start(&self.spawner, &target.path).await.unwrap_or_else(|err| {
            tracing::warn!("benchmark of version [{}] is timed from zero: {err}", target.version.0);
            0
        });
        let _ = tokio::fs::create_dir_all(&self.scratch).await;
        for (index, run) in picked {
            for encoder in &encoders {
                let run = self.run_one(&spec, index, window(&run, start_us), encoder).await;
                self.update(|status| status.runs.push(run));
            }
        }
        let _ = tokio::fs::remove_dir_all(&self.scratch).await;
        self.update(|status| {
            status.running = false;
            status.finished_at = Some(Timestamp::now());
        });
        tracing::info!("benchmarked version [{}]", target.version.0);
    }

    async fn run_one(
        &self,
        spec: &TranscodeSpec,
        index: usize,
        window: RunWindow,
        encoder: &VideoEncoder,
    ) -> BenchmarkRun {
        let name = if encoder.is_hardware() { "vaapi" } else { "software" };
        let out = self.scratch.join(format!("run_{index}_{name}"));
        let _ = tokio::fs::create_dir_all(&out).await;
        let args = build_run_args(spec, encoder, window, 0.0, &out.to_string_lossy(), None);
        let started = Instant::now();
        let outcome =
            match tokio::time::timeout(self.cap, self.spawner.run_captured(BINARY, &args)).await {
                Err(_) => BenchmarkOutcome::TooSlow,
                Ok(Err(error)) => BenchmarkOutcome::Failed { detail: error.to_string() },
                Ok(Ok(output)) if output.success => BenchmarkOutcome::Ok,
                Ok(Ok(output)) => BenchmarkOutcome::Failed { detail: output.failure_detail(5) },
            };
        let elapsed_ms = millis(started.elapsed());
        let _ = tokio::fs::remove_dir_all(&out).await;
        BenchmarkRun {
            segment: index,
            start_ms: window.origin_us / 1_000,
            duration_ms: window.length_ms.unwrap_or_default(),
            hardware: encoder.is_hardware(),
            outcome,
            elapsed_ms,
        }
    }
}

struct Running<'a>(&'a Mutex<BenchmarkStatus>);

impl Drop for Running<'_> {
    fn drop(&mut self) {
        if let Ok(mut status) = self.0.lock() {
            status.running = false;
            status.finished_at.get_or_insert_with(Timestamp::now);
        }
    }
}

fn picked(plan: &SegmentPlan, target: &BenchmarkTarget) -> Vec<(usize, Segment)> {
    let mut indices: Vec<usize> = Vec::new();
    for quarter in QUARTERS {
        let index = plan.index_at(target.duration_ms * quarter / 4);
        if !indices.contains(&index) {
            indices.push(index);
        }
    }
    indices
        .into_iter()
        .filter_map(|index| {
            let start_ms = plan.segments.get(index)?.start_ms;
            let duration_ms = (RUN_SEGMENTS * TARGET_MS).min(target.duration_ms - start_ms);
            Some((index, Segment { start_ms, duration_ms }))
        })
        .collect()
}

fn window(run: &Segment, start_us: i64) -> RunWindow {
    let segments = usize::try_from(run.duration_ms.div_ceil(TARGET_MS)).unwrap_or(usize::MAX);
    RunWindow {
        origin_us: run.start_ms * 1_000,
        start_us,
        segments,
        length_ms: Some(run.duration_ms),
    }
}

fn spec_for(target: &BenchmarkTarget, max_height: Option<u32>) -> TranscodeSpec {
    let scale_to = match (target.height, max_height) {
        (Some(height), Some(cap)) if height > cap => Some(cap),
        _ => None,
    };
    TranscodeSpec {
        session: SessionId("benchmark".to_owned()),
        generation: StreamGeneration(0),
        version: target.version.clone(),
        input_path: target.path.clone(),
        duration_ms: target.duration_ms,
        keyframes_us: None,
        copy: false,
        container: SegmentContainer::MpegTs,
        seek_ms: None,
        audio_track: target.audio_track,
        max_height: scale_to,
        max_bitrate: None,
        burn_subtitle: None,
        soft_subtitle: None,
        convert_audio: false,
        audio_channels: None,
        source_hdr: target.hdr,
    }
}

fn millis(elapsed: Duration) -> u64 {
    u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use std::io;

    use domain::catalog::VersionId;
    use domain::media::HdrFormat;
    use domain::process::CommandOutput;

    use super::*;

    #[derive(Default)]
    struct Recorder {
        vaapi_fails: bool,
        stall: bool,
        missing: bool,
        panics: bool,
        start: &'static str,
        calls: Mutex<Vec<Vec<String>>>,
    }

    impl ProcessSpawner for Recorder {
        async fn run(&self, _program: &str, _args: &[String]) -> io::Result<bool> {
            unreachable!("the benchmark always captures output")
        }

        async fn run_captured(&self, program: &str, args: &[String]) -> io::Result<CommandOutput> {
            if program == "ffprobe" {
                if self.missing {
                    return Err(io::Error::new(io::ErrorKind::NotFound, "No such file"));
                }
                let stdout = self.start.to_owned();
                return Ok(CommandOutput { success: true, stdout, ..CommandOutput::default() });
            }
            assert_eq!(program, "ffmpeg");
            assert!(!self.panics, "the encoder crashed");
            self.calls.lock().unwrap().push(args.to_vec());
            if self.missing {
                return Err(io::Error::new(io::ErrorKind::NotFound, "No such file or directory"));
            }
            if self.stall {
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            let vaapi = args.contains(&"h264_vaapi".to_owned());
            let failing = vaapi && self.vaapi_fails;
            Ok(CommandOutput {
                success: !failing,
                stdout: String::new(),
                stderr: if failing {
                    "No usable encoding entrypoint".to_owned()
                } else {
                    String::new()
                },
                status: String::new(),
            })
        }
    }

    fn target() -> BenchmarkTarget {
        BenchmarkTarget {
            version: VersionId("v1".to_owned()),
            path: "/media/film.mkv".to_owned(),
            duration_ms: 100_000,
            codec: Some("hevc".to_owned()),
            width: Some(3840),
            height: Some(2160),
            hdr: Some(HdrFormat::Hdr10),
            audio_track: Some(1),
        }
    }

    fn bench(
        recorder: Recorder,
        encoder: VideoEncoder,
        cap: Duration,
        scratch: &std::path::Path,
    ) -> SegmentBenchmark<Recorder> {
        SegmentBenchmark::with_parts(recorder, scratch, encoder, Some(1080), cap)
    }

    async fn finished<S: ProcessSpawner + 'static>(
        benchmark: &SegmentBenchmark<S>,
    ) -> BenchmarkStatus {
        loop {
            let status = benchmark.status();
            if !status.running {
                return status;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    fn vaapi() -> VideoEncoder {
        VideoEncoder::Vaapi { device: "/dev/dri/renderD128".to_owned() }
    }

    #[tokio::test]
    async fn a_short_run_from_each_quarter_point_runs_on_each_encoder() {
        let scratch = tempfile::tempdir().unwrap();
        let benchmark = bench(Recorder::default(), vaapi(), RUN_CAP, &scratch.path().join("bench"));

        benchmark.start(target()).unwrap();
        let status = finished(&benchmark).await;

        let runs: Vec<(usize, u64, bool)> =
            status.runs.iter().map(|run| (run.segment, run.start_ms, run.hardware)).collect();
        assert_eq!(
            runs,
            [
                (6, 24_000, true),
                (6, 24_000, false),
                (12, 48_000, true),
                (12, 48_000, false),
                (18, 72_000, true),
                (18, 72_000, false)
            ]
        );
        assert!(status.runs.iter().all(|run| run.outcome == BenchmarkOutcome::Ok));
        assert!(status.runs.iter().all(|run| run.duration_ms == 12_000));
        assert_eq!(status.total, 6);
        assert!(status.finished_at.is_some());
        assert!(!scratch.path().join("bench").exists(), "the scratch folder is removed");
    }

    #[tokio::test]
    async fn each_run_is_the_command_playback_uses_for_three_segments() {
        let scratch = tempfile::tempdir().unwrap();
        let recorder = Recorder { start: "1.400000\n", ..Recorder::default() };
        let benchmark = bench(recorder, vaapi(), RUN_CAP, scratch.path());

        benchmark.start(target()).unwrap();
        finished(&benchmark).await;

        let calls = benchmark.inner.spawner.calls.lock().unwrap().clone();
        let spec = spec_for(&target(), Some(1080));
        let out = scratch.path().join("run_6_vaapi");
        let window = RunWindow {
            origin_us: 24_000_000,
            start_us: 1_400_000,
            segments: 3,
            length_ms: Some(12_000),
        };
        let expected = build_run_args(&spec, &vaapi(), window, 0.0, &out.to_string_lossy(), None);
        assert_eq!(calls[0], expected);
        let filter = calls[1].iter().skip_while(|arg| *arg != "-vf").nth(1).unwrap();
        assert!(
            filter.starts_with("trim=start=25.400000:end=37.400000,scale=-2:1080,"),
            "{filter}"
        );
        assert!(filter.contains("tonemap"), "{filter}");
        assert!(calls[1].contains(&"libx264".to_owned()));
        assert!(calls[1].contains(&"0:1".to_owned()), "the first audio track is mapped");
        assert!(!calls[1].contains(&"-readrate".to_owned()));
    }

    #[tokio::test]
    async fn software_alone_runs_when_no_hardware_is_in_use() {
        let scratch = tempfile::tempdir().unwrap();
        let benchmark = bench(Recorder::default(), VideoEncoder::Software, RUN_CAP, scratch.path());

        benchmark.start(target()).unwrap();
        let status = finished(&benchmark).await;

        assert_eq!(status.total, 3);
        assert!(status.runs.iter().all(|run| !run.hardware));
    }

    #[tokio::test]
    async fn a_hardware_failure_is_reported_not_hidden() {
        let scratch = tempfile::tempdir().unwrap();
        let recorder = Recorder { vaapi_fails: true, ..Recorder::default() };
        let benchmark = bench(recorder, vaapi(), RUN_CAP, scratch.path());

        benchmark.start(target()).unwrap();
        let status = finished(&benchmark).await;

        assert_eq!(
            status.runs[0].outcome,
            BenchmarkOutcome::Failed { detail: "No usable encoding entrypoint".to_owned() }
        );
        assert_eq!(status.runs[1].outcome, BenchmarkOutcome::Ok);
    }

    #[tokio::test]
    async fn a_run_whose_ffmpeg_cannot_start_fails_with_the_reason() {
        let scratch = tempfile::tempdir().unwrap();
        let recorder = Recorder { missing: true, ..Recorder::default() };
        let benchmark = bench(recorder, VideoEncoder::Software, RUN_CAP, scratch.path());

        benchmark.start(target()).unwrap();
        let status = finished(&benchmark).await;

        assert!(status.runs.iter().all(|run| run.outcome
            == BenchmarkOutcome::Failed { detail: "No such file or directory".to_owned() }));
    }

    #[tokio::test]
    async fn a_run_past_the_cap_is_too_slow() {
        let scratch = tempfile::tempdir().unwrap();
        let recorder = Recorder { stall: true, ..Recorder::default() };
        let benchmark =
            bench(recorder, VideoEncoder::Software, Duration::from_millis(20), scratch.path());

        benchmark.start(target()).unwrap();
        let status = finished(&benchmark).await;

        assert!(status.runs.iter().all(|run| run.outcome == BenchmarkOutcome::TooSlow));
    }

    #[tokio::test]
    async fn a_benchmark_that_crashes_is_not_left_running() {
        let scratch = tempfile::tempdir().unwrap();
        let recorder = Recorder { panics: true, ..Recorder::default() };
        let benchmark = bench(recorder, VideoEncoder::Software, RUN_CAP, scratch.path());

        benchmark.start(target()).unwrap();
        let status = finished(&benchmark).await;

        assert!(status.finished_at.is_some());
        assert!(status.runs.is_empty());
        assert_eq!(benchmark.start(target()), Ok(()));
        finished(&benchmark).await;
    }

    #[tokio::test]
    async fn a_second_start_while_running_is_refused() {
        let scratch = tempfile::tempdir().unwrap();
        let recorder = Recorder { stall: true, ..Recorder::default() };
        let benchmark =
            bench(recorder, VideoEncoder::Software, Duration::from_millis(20), scratch.path());

        benchmark.start(target()).unwrap();
        let refused = benchmark.clone().start(target());
        let status = finished(&benchmark).await;

        assert_eq!(refused, Err(BenchmarkError::Running));
        assert_eq!(status.total, 3, "the first run carried on");
        assert!(benchmark.start(target()).is_ok(), "a finished benchmark can run again");
        finished(&benchmark).await;
    }

    #[test]
    fn a_source_within_the_cap_is_not_scaled() {
        let small = BenchmarkTarget { height: Some(720), hdr: None, ..target() };

        assert_eq!(spec_for(&small, Some(1080)).max_height, None);
        assert_eq!(spec_for(&target(), None).max_height, None);
        assert_eq!(spec_for(&target(), Some(1080)).max_height, Some(1080));
    }

    fn picks(duration_ms: u64) -> Vec<(usize, u64, u64)> {
        let short = BenchmarkTarget { duration_ms, ..target() };
        picked(&plan_fixed_segments(duration_ms, TARGET_MS), &short)
            .into_iter()
            .map(|(index, run)| (index, run.start_ms, run.duration_ms))
            .collect()
    }

    #[test]
    fn a_short_film_runs_each_start_once_and_only_what_is_left() {
        assert_eq!(picks(6_000), [(0, 0, 6_000)]);
        assert_eq!(picks(10_000), [(0, 0, 10_000), (1, 4_000, 6_000)]);
        assert_eq!(window(&Segment { start_ms: 4_000, duration_ms: 6_000 }, 0).segments, 2);
    }

    #[test]
    fn a_film_with_no_segments_picks_none() {
        assert!(picks(0).is_empty());
    }
}

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use domain::diagnostics::{
    BenchmarkOutcome, BenchmarkRun, BenchmarkStatus, BenchmarkTarget, TranscodeBenchmark,
};
use domain::error::BenchmarkError;
use domain::media::KeyframeProbe;
use domain::process::ProcessSpawner;
use domain::session::{
    Segment, SegmentContainer, SegmentPlan, SessionId, StreamGeneration, TranscodeSpec,
    plan_segments,
};
use jiff::Timestamp;

use crate::probe::FfprobeMediaProbe;
use crate::transcode::{TARGET_MS, TokioProcessSpawner, VideoEncoder, build_segment_args};

const BINARY: &str = "ffmpeg";
const RUN_CAP: Duration = Duration::from_secs(60);
const KEYFRAME_LIMIT: Duration = Duration::from_secs(600);
const QUARTERS: [u64; 3] = [1, 2, 3];

pub struct SegmentBenchmark<S = TokioProcessSpawner, P = FfprobeMediaProbe> {
    inner: Arc<Inner<S, P>>,
}

struct Inner<S, P> {
    spawner: S,
    prober: P,
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
        Self::with_parts(
            TokioProcessSpawner,
            FfprobeMediaProbe::default(),
            scratch,
            encoder,
            max_height,
            RUN_CAP,
        )
    }
}

impl<S, P> SegmentBenchmark<S, P> {
    pub fn with_parts(
        spawner: S,
        prober: P,
        scratch: impl Into<PathBuf>,
        encoder: VideoEncoder,
        max_height: Option<u32>,
        cap: Duration,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                spawner,
                prober,
                scratch: scratch.into(),
                encoder,
                max_height,
                cap,
                status: Mutex::default(),
            }),
        }
    }
}

impl<S, P> Clone for SegmentBenchmark<S, P> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<S: ProcessSpawner + 'static, P: KeyframeProbe + 'static> TranscodeBenchmark
    for SegmentBenchmark<S, P>
{
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

impl<S: ProcessSpawner, P: KeyframeProbe> Inner<S, P> {
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
        let started = Instant::now();
        let read = tokio::time::timeout(KEYFRAME_LIMIT, self.prober.keyframes(&target.path)).await;
        let (keyframes, keyframes_error) = match read {
            Ok(Ok(keyframes)) => (keyframes, None),
            Ok(Err(error)) => (Vec::new(), Some(error.to_string())),
            Err(_) => (Vec::new(), Some(keyframe_limit_message())),
        };
        let keyframes_ms = millis(started.elapsed());
        let picked = picked(&plan_segments(&keyframes, target.duration_ms, TARGET_MS), &target);
        let encoders = self.encoders();
        self.update(|status| {
            status.keyframes_ms = Some(keyframes_ms);
            status.keyframes_error = keyframes_error;
            status.total = picked.len() * encoders.len();
        });
        let spec = spec_for(&target, self.max_height);
        let _ = tokio::fs::create_dir_all(&self.scratch).await;
        for (index, segment) in picked {
            for encoder in &encoders {
                let run = self.run_one(&spec, index, &segment, encoder).await;
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
        segment: &Segment,
        encoder: &VideoEncoder,
    ) -> BenchmarkRun {
        let name = if encoder.is_hardware() { "vaapi" } else { "software" };
        let out = self.scratch.join(format!("segment_{index}_{name}.ts"));
        let args = build_segment_args(spec, segment, &out.to_string_lossy(), encoder);
        let started = Instant::now();
        let outcome =
            match tokio::time::timeout(self.cap, self.spawner.run_captured(BINARY, &args)).await {
                Err(_) => BenchmarkOutcome::TooSlow,
                Ok(Err(error)) => BenchmarkOutcome::Failed { detail: error.to_string() },
                Ok(Ok(output)) if output.success => BenchmarkOutcome::Ok,
                Ok(Ok(output)) => BenchmarkOutcome::Failed { detail: output.failure_detail(5) },
            };
        let elapsed_ms = millis(started.elapsed());
        let _ = tokio::fs::remove_file(&out).await;
        BenchmarkRun {
            segment: index,
            start_ms: segment.start_ms,
            duration_ms: segment.duration_ms,
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

fn keyframe_limit_message() -> String {
    format!("the keyframe read did not finish within {} minutes", KEYFRAME_LIMIT.as_secs() / 60)
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
        .filter_map(|index| plan.segments.get(index).map(|segment| (index, segment.clone())))
        .collect()
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
        copy: false,
        container: SegmentContainer::MpegTs,
        seek_ms: None,
        audio_track: target.audio_track,
        max_height: scale_to,
        max_bitrate: None,
        burn_subtitle_path: None,
        soft_subtitle: None,
        downmix_stereo: false,
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
    use domain::error::ProbeError;
    use domain::media::HdrFormat;
    use domain::process::CommandOutput;

    use super::*;

    struct Keyframes(Result<Vec<u64>, String>);

    impl KeyframeProbe for Keyframes {
        async fn keyframes(&self, _path: &str) -> Result<Vec<u64>, ProbeError> {
            self.0.clone().map_err(ProbeError::Backend)
        }
    }

    #[derive(Default)]
    struct Recorder {
        vaapi_fails: bool,
        stall: bool,
        missing: bool,
        panics: bool,
        calls: Mutex<Vec<Vec<String>>>,
    }

    impl ProcessSpawner for Recorder {
        async fn run(&self, _program: &str, _args: &[String]) -> io::Result<bool> {
            unreachable!("the benchmark always captures output")
        }

        async fn run_captured(&self, program: &str, args: &[String]) -> io::Result<CommandOutput> {
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

    fn every_two_seconds() -> Keyframes {
        Keyframes(Ok((0..50).map(|n| n * 2_000).collect()))
    }

    fn bench(
        recorder: Recorder,
        keyframes: Keyframes,
        encoder: VideoEncoder,
        cap: Duration,
        scratch: &std::path::Path,
    ) -> SegmentBenchmark<Recorder, Keyframes> {
        SegmentBenchmark::with_parts(recorder, keyframes, scratch, encoder, Some(1080), cap)
    }

    async fn finished<S: ProcessSpawner + 'static, P: KeyframeProbe + 'static>(
        benchmark: &SegmentBenchmark<S, P>,
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
    async fn three_segments_spread_through_the_film_run_on_each_encoder() {
        let scratch = tempfile::tempdir().unwrap();
        let benchmark = bench(
            Recorder::default(),
            every_two_seconds(),
            vaapi(),
            RUN_CAP,
            &scratch.path().join("bench"),
        );

        benchmark.start(target()).unwrap();
        let status = finished(&benchmark).await;

        let runs: Vec<(u64, bool)> =
            status.runs.iter().map(|run| (run.start_ms, run.hardware)).collect();
        assert_eq!(
            runs,
            [
                (24_000, true),
                (24_000, false),
                (48_000, true),
                (48_000, false),
                (72_000, true),
                (72_000, false)
            ]
        );
        assert!(status.runs.iter().all(|run| run.outcome == BenchmarkOutcome::Ok));
        assert!(status.runs.iter().all(|run| run.duration_ms == 4_000));
        assert_eq!(status.total, 6);
        assert!(status.keyframes_ms.is_some());
        assert_eq!(status.keyframes_error, None);
        assert!(status.finished_at.is_some());
        assert!(!scratch.path().join("bench").exists(), "the scratch folder is removed");
    }

    #[tokio::test]
    async fn each_run_is_the_command_playback_uses() {
        let scratch = tempfile::tempdir().unwrap();
        let benchmark =
            bench(Recorder::default(), every_two_seconds(), vaapi(), RUN_CAP, scratch.path());

        benchmark.start(target()).unwrap();
        finished(&benchmark).await;

        let calls = benchmark.inner.spawner.calls.lock().unwrap().clone();
        let spec = spec_for(&target(), Some(1080));
        let segment = Segment { start_ms: 24_000, duration_ms: 4_000 };
        let out = scratch.path().join("segment_6_vaapi.ts");
        assert_eq!(calls[0], build_segment_args(&spec, &segment, &out.to_string_lossy(), &vaapi()));
        let filter = calls[1].iter().skip_while(|arg| *arg != "-vf").nth(1).unwrap();
        assert!(filter.starts_with("scale=-2:1080,"), "{filter}");
        assert!(filter.contains("tonemap"), "{filter}");
        assert!(calls[1].contains(&"libx264".to_owned()));
        assert!(calls[1].contains(&"0:1".to_owned()), "the first audio track is mapped");
    }

    #[tokio::test]
    async fn software_alone_runs_when_no_hardware_is_in_use() {
        let scratch = tempfile::tempdir().unwrap();
        let benchmark = bench(
            Recorder::default(),
            every_two_seconds(),
            VideoEncoder::Software,
            RUN_CAP,
            scratch.path(),
        );

        benchmark.start(target()).unwrap();
        let status = finished(&benchmark).await;

        assert_eq!(status.total, 3);
        assert!(status.runs.iter().all(|run| !run.hardware));
    }

    #[tokio::test]
    async fn a_hardware_failure_is_reported_not_hidden() {
        let scratch = tempfile::tempdir().unwrap();
        let recorder = Recorder { vaapi_fails: true, ..Recorder::default() };
        let benchmark = bench(recorder, every_two_seconds(), vaapi(), RUN_CAP, scratch.path());

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
        let benchmark =
            bench(recorder, every_two_seconds(), VideoEncoder::Software, RUN_CAP, scratch.path());

        benchmark.start(target()).unwrap();
        let status = finished(&benchmark).await;

        assert!(status.runs.iter().all(|run| run.outcome
            == BenchmarkOutcome::Failed { detail: "No such file or directory".to_owned() }));
    }

    #[tokio::test]
    async fn a_run_past_the_cap_is_too_slow() {
        let scratch = tempfile::tempdir().unwrap();
        let recorder = Recorder { stall: true, ..Recorder::default() };
        let benchmark = bench(
            recorder,
            every_two_seconds(),
            VideoEncoder::Software,
            Duration::from_millis(20),
            scratch.path(),
        );

        benchmark.start(target()).unwrap();
        let status = finished(&benchmark).await;

        assert!(status.runs.iter().all(|run| run.outcome == BenchmarkOutcome::TooSlow));
    }

    #[tokio::test]
    async fn a_file_without_keyframes_is_one_long_segment_and_says_why() {
        let scratch = tempfile::tempdir().unwrap();
        let benchmark = bench(
            Recorder::default(),
            Keyframes(Err("ffprobe failed".to_owned())),
            VideoEncoder::Software,
            RUN_CAP,
            scratch.path(),
        );

        benchmark.start(target()).unwrap();
        let status = finished(&benchmark).await;

        assert_eq!(status.runs.len(), 1);
        assert_eq!(status.runs[0].duration_ms, 100_000);
        assert!(status.keyframes_error.as_deref().is_some_and(|e| e.contains("ffprobe failed")));
    }

    struct StalledKeyframes;

    impl KeyframeProbe for StalledKeyframes {
        async fn keyframes(&self, _path: &str) -> Result<Vec<u64>, ProbeError> {
            std::future::pending().await
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_keyframe_read_that_never_finishes_gives_up_at_its_limit() {
        let scratch = tempfile::tempdir().unwrap();
        let benchmark = SegmentBenchmark::with_parts(
            Recorder::default(),
            StalledKeyframes,
            scratch.path(),
            VideoEncoder::Software,
            Some(1080),
            RUN_CAP,
        );

        benchmark.start(target()).unwrap();
        let status = finished(&benchmark).await;

        assert_eq!(
            status.keyframes_error.as_deref(),
            Some("the keyframe read did not finish within 10 minutes")
        );
        assert_eq!(status.runs.len(), 1);
        assert!(status.finished_at.is_some());
    }

    #[tokio::test]
    async fn a_benchmark_that_crashes_is_not_left_running() {
        let scratch = tempfile::tempdir().unwrap();
        let recorder = Recorder { panics: true, ..Recorder::default() };
        let benchmark =
            bench(recorder, every_two_seconds(), VideoEncoder::Software, RUN_CAP, scratch.path());

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
        let benchmark = bench(
            recorder,
            every_two_seconds(),
            VideoEncoder::Software,
            Duration::from_millis(20),
            scratch.path(),
        );

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

    #[test]
    fn a_short_film_picks_each_segment_once() {
        let plan = plan_segments(&[0, 4_000], 6_000, TARGET_MS);
        let short = BenchmarkTarget { duration_ms: 6_000, ..target() };

        let picked = picked(&plan, &short);

        assert_eq!(picked.iter().map(|(index, _)| *index).collect::<Vec<_>>(), [0, 1]);
    }

    #[test]
    fn a_film_with_no_segments_picks_none() {
        let empty = BenchmarkTarget { duration_ms: 0, ..target() };

        assert!(picked(&plan_segments(&[], 0, TARGET_MS), &empty).is_empty());
    }
}

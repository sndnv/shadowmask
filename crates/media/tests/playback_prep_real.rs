use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use domain::catalog::VersionId;
use domain::media::{
    EmbeddedSubtitleTrack, KeyframeProbe, PlaybackPrep, PlaybackPreparer, SubtitleFormat,
};
use domain::process::{CommandOutput, ProcessSpawner};
use media::playback_prep::FfmpegPlaybackPreparer;
use media::playback_prep_store::FsPlaybackPrepStore;
use media::probe::FfprobeMediaProbe;
use media::transcode::TokioProcessSpawner;

type Instead = fn(&[String]) -> Option<io::Result<CommandOutput>>;
type After = fn(&[String]);

#[derive(Clone, Default)]
struct Scripted {
    runs: Arc<Mutex<Vec<Vec<String>>>>,
    instead: Option<Instead>,
    after: Option<After>,
}

impl Scripted {
    fn runs(&self) -> usize {
        self.runs.lock().unwrap().len()
    }

    fn reads(&self) -> Vec<Vec<String>> {
        self.runs
            .lock()
            .unwrap()
            .iter()
            .filter(|args| args.iter().any(|arg| arg == "framecrc"))
            .cloned()
            .collect()
    }

    fn read(&self) -> Vec<String> {
        self.reads().into_iter().next().expect("the file was read")
    }
}

impl ProcessSpawner for Scripted {
    async fn run(&self, program: &str, args: &[String]) -> io::Result<bool> {
        TokioProcessSpawner.run(program, args).await
    }

    async fn run_captured(&self, program: &str, args: &[String]) -> io::Result<CommandOutput> {
        self.runs.lock().unwrap().push(args.to_vec());
        if let Some(instead) = self.instead
            && let Some(outcome) = instead(args)
        {
            return outcome;
        }
        let outcome = TokioProcessSpawner.run_captured(program, args).await;
        if let Some(after) = self.after {
            after(args);
        }
        outcome
    }
}

fn fixture(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn track(index: u32, format: SubtitleFormat) -> EmbeddedSubtitleTrack {
    EmbeddedSubtitleTrack { index, language: None, format, forced: false, default: false }
}

fn version() -> VersionId {
    VersionId("ver-1".to_owned())
}

fn preparer(root: &Path, spawner: Scripted) -> FfmpegPlaybackPreparer<Scripted> {
    FfmpegPlaybackPreparer::with_spawner(FsPlaybackPrepStore::new(root), spawner)
}

async fn probed_keyframes(name: &str) -> Vec<u64> {
    FfprobeMediaProbe::default()
        .keyframes(&fixture(name))
        .await
        .expect("ffprobe should list keyframes; is ffmpeg installed and on PATH?")
}

fn stored(root: &Path, prefix: &str, extension: &str) -> Vec<PathBuf> {
    let dir = root.join("ver-1");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            name.starts_with(&format!("{prefix}-")) && name.ends_with(&format!(".{extension}"))
        })
        .collect()
}

fn refused(detail: &str) -> Option<io::Result<CommandOutput>> {
    Some(Ok(CommandOutput {
        success: false,
        stderr: detail.to_owned(),
        status: "exit code 1".to_owned(),
        ..CommandOutput::default()
    }))
}

fn output_after(args: &[String], format: &str) -> Option<String> {
    let at = args.windows(2).position(|pair| pair[0] == "-f" && pair[1] == format)?;
    args.get(at + 2).cloned()
}

#[tokio::test]
async fn one_read_gives_the_keyframes_and_every_text_track() {
    let root = tempfile::tempdir().expect("tempdir");
    let spawner = Scripted::default();
    let prep = preparer(root.path(), spawner.clone())
        .prepare(&version(), &fixture("sample_full.mkv"), &[track(2, SubtitleFormat::Srt)])
        .await
        .expect("ffmpeg should prepare the fixture; is ffmpeg installed and on PATH?");

    assert_eq!(prep.keyframes, probed_keyframes("sample_full.mkv").await);
    assert_eq!(prep.extracted_subtitles, vec![2]);
    assert_eq!(spawner.runs(), 3, "a header probe, one read of the file, one conversion of a copy");
    let read = spawner.read();
    assert!(read.windows(2).any(|pair| pair == ["-f", "framecrc"]));
    assert!(read.windows(2).any(|pair| pair == ["-f", "matroska"]));
    assert_eq!(stored(root.path(), "embedded-2", "mks").len(), 1);
    let vtt = stored(root.path(), "embedded-2", "vtt");
    assert_eq!(vtt.len(), 1);
    assert!(std::fs::read_to_string(&vtt[0]).unwrap().starts_with("WEBVTT"));
    assert_eq!(stored(root.path(), "keyframes", "json").len(), 1);
}

#[tokio::test]
async fn a_prepared_version_is_not_read_again() {
    let root = tempfile::tempdir().expect("tempdir");
    let spawner = Scripted::default();
    let subject = preparer(root.path(), spawner.clone());
    let tracks = [track(2, SubtitleFormat::Srt)];
    let first =
        subject.prepare(&version(), &fixture("sample_full.mkv"), &tracks).await.expect("prepare");
    let runs = spawner.runs();

    let again =
        subject.prepare(&version(), &fixture("sample_full.mkv"), &tracks).await.expect("prepare");

    assert_eq!(again, first);
    assert_eq!(spawner.runs(), runs + 1, "a cached result only probes the header");
}

#[tokio::test]
async fn an_image_track_is_copied_and_gets_no_webvtt() {
    let root = tempfile::tempdir().expect("tempdir");
    let spawner = Scripted::default();
    let prep = preparer(root.path(), spawner.clone())
        .prepare(&version(), &fixture("supsample.mkv"), &[track(1, SubtitleFormat::Pgs)])
        .await
        .expect("prepare");

    assert_eq!(prep.keyframes, probed_keyframes("supsample.mkv").await);
    assert_eq!(prep.extracted_subtitles, vec![1]);
    assert_eq!(spawner.runs(), 1);
    assert_eq!(stored(root.path(), "embedded-1", "mks").len(), 1);
    assert!(stored(root.path(), "embedded-1", "vtt").is_empty());
}

#[tokio::test]
async fn a_track_that_cannot_be_copied_still_leaves_the_keyframes() {
    let root = tempfile::tempdir().expect("tempdir");
    let spawner = Scripted::default();
    let prep = preparer(root.path(), spawner.clone())
        .prepare(&version(), &fixture("sample_full.mkv"), &[track(9, SubtitleFormat::Pgs)])
        .await
        .expect("prepare");

    assert_eq!(prep.keyframes, probed_keyframes("sample_full.mkv").await);
    assert!(prep.extracted_subtitles.is_empty());
    assert_eq!(spawner.runs(), 2, "the read is repeated for the keyframes alone");
    assert!(stored(root.path(), "embedded-9", "mks").is_empty());
    let leftovers: Vec<PathBuf> = std::fs::read_dir(root.path().join("ver-1"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "part" || ext == "crc"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}

#[tokio::test]
async fn a_text_track_that_cannot_become_webvtt_is_left_out_and_tried_again() {
    let root = tempfile::tempdir().expect("tempdir");
    let spawner = Scripted {
        instead: Some(|args| {
            args.iter().any(|arg| arg == "webvtt").then(|| refused("no webvtt today")).flatten()
        }),
        ..Scripted::default()
    };
    let subject = preparer(root.path(), spawner.clone());
    let tracks = [track(2, SubtitleFormat::Srt)];

    let prep =
        subject.prepare(&version(), &fixture("sample_full.mkv"), &tracks).await.expect("prepare");
    assert!(prep.extracted_subtitles.is_empty());
    assert!(!prep.keyframes.is_empty());
    assert!(stored(root.path(), "embedded-2", "vtt").is_empty());
    let runs = spawner.runs();

    subject.prepare(&version(), &fixture("sample_full.mkv"), &tracks).await.expect("prepare");
    assert!(spawner.runs() > runs, "a track that was left out makes the next run read again");
    let copies: Vec<String> =
        spawner.reads().iter().filter_map(|args| output_after(args, "matroska")).collect();
    assert_eq!(copies.len(), 2);
    assert_ne!(copies[0], copies[1], "each run writes its own temporary copy");
}

#[tokio::test]
async fn a_conversion_that_cannot_start_is_left_out() {
    let root = tempfile::tempdir().expect("tempdir");
    let spawner = Scripted {
        instead: Some(|args| {
            args.iter().any(|arg| arg == "webvtt").then(|| Err(io::Error::other("no such binary")))
        }),
        ..Scripted::default()
    };
    let prep = preparer(root.path(), spawner)
        .prepare(&version(), &fixture("sample_full.mkv"), &[track(2, SubtitleFormat::Srt)])
        .await
        .expect("prepare");

    assert!(prep.extracted_subtitles.is_empty());
}

#[tokio::test]
async fn a_copy_that_vanishes_before_it_is_kept_is_left_out() {
    let root = tempfile::tempdir().expect("tempdir");
    let spawner = Scripted {
        after: Some(|args| {
            if let Some(copy) = output_after(args, "matroska") {
                let _ = std::fs::remove_file(copy);
            }
        }),
        ..Scripted::default()
    };
    let prep = preparer(root.path(), spawner)
        .prepare(&version(), &fixture("sample_full.mkv"), &[track(2, SubtitleFormat::Srt)])
        .await
        .expect("prepare");

    assert!(prep.extracted_subtitles.is_empty());
    assert!(!prep.keyframes.is_empty());
}

#[tokio::test]
async fn keyframes_that_cannot_be_kept_fail_the_preparation() {
    let root = tempfile::tempdir().expect("tempdir");
    let spawner = Scripted {
        after: Some(|args| {
            if let Some(crc) = output_after(args, "framecrc") {
                let (kept, _run) = crc
                    .strip_suffix(".crc")
                    .and_then(|rest| rest.rsplit_once('.'))
                    .expect("the listing sits beside the keyframes");
                std::fs::create_dir_all(kept).expect("block the keyframes with a directory");
            }
        }),
        ..Scripted::default()
    };
    let result: Result<PlaybackPrep, _> =
        preparer(root.path(), spawner).prepare(&version(), &fixture("sample_full.mkv"), &[]).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn a_file_that_cannot_be_read_fails() {
    let root = tempfile::tempdir().expect("tempdir");
    let spawner = Scripted::default();
    let without_tracks = preparer(root.path(), spawner.clone())
        .prepare(&version(), "/nonexistent/shadowmask/movie.mkv", &[])
        .await;
    assert!(without_tracks.is_err());
    assert_eq!(spawner.runs(), 1);

    let with_tracks = preparer(root.path(), Scripted::default())
        .prepare(&version(), "/nonexistent/shadowmask/movie.mkv", &[track(2, SubtitleFormat::Srt)])
        .await;
    assert!(with_tracks.is_err());
}

#[tokio::test]
async fn a_read_that_cannot_start_fails() {
    let root = tempfile::tempdir().expect("tempdir");
    let spawner = Scripted {
        instead: Some(|_| Some(Err(io::Error::other("no ffmpeg")))),
        ..Default::default()
    };

    let result =
        preparer(root.path(), spawner).prepare(&version(), &fixture("sample_full.mkv"), &[]).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn a_probe_that_cannot_start_fails_before_the_read() {
    let root = tempfile::tempdir().expect("tempdir");
    let spawner = Scripted {
        instead: Some(|args| {
            args.iter()
                .any(|arg| arg == "-show_entries")
                .then(|| Err(io::Error::other("no ffprobe")))
        }),
        ..Scripted::default()
    };

    let result = preparer(root.path(), spawner.clone())
        .prepare(&version(), &fixture("sample_full.mkv"), &[track(2, SubtitleFormat::Srt)])
        .await;

    assert!(result.is_err());
    assert_eq!(spawner.runs(), 1);
}

fn offset_mp4(dir: &Path) -> String {
    let cues = dir.join("cues.srt");
    std::fs::write(&cues, "1\n00:00:30,000 --> 00:00:32,000\nFirst\n").expect("write cues");
    let mp4 = dir.join("offset.mp4");
    let built = std::process::Command::new("ffmpeg")
        .args(["-nostdin", "-v", "error", "-y", "-f", "lavfi"])
        .args(["-i", "color=c=black:s=64x64:r=2:d=35", "-i"])
        .arg(&cues)
        .args(["-map", "0", "-map", "1", "-c:v", "mpeg4", "-c:s", "mov_text"])
        .args(["-output_ts_offset", "10"])
        .arg(&mp4)
        .status()
        .expect("ffmpeg should build the fixture; is ffmpeg installed and on PATH?");
    assert!(built.success());
    mp4.to_string_lossy().into_owned()
}

#[tokio::test]
async fn cues_keep_their_time_in_a_file_that_does_not_start_at_zero() {
    let source = tempfile::tempdir().expect("tempdir");
    let input = offset_mp4(source.path());
    let root = tempfile::tempdir().expect("tempdir");

    let prep = preparer(root.path(), Scripted::default())
        .prepare(&version(), &input, &[track(1, SubtitleFormat::Srt)])
        .await
        .expect("prepare");

    assert_eq!(prep.extracted_subtitles, vec![1]);
    let vtt = stored(root.path(), "embedded-1", "vtt");
    let cues = std::fs::read_to_string(&vtt[0]).unwrap();
    assert!(cues.contains("00:30.000 --> 00:32.000"), "{cues}");
}

fn dvb_beside_srt(dir: &Path) -> String {
    let cues = dir.join("cues.srt");
    std::fs::write(&cues, "1\n00:00:01,000 --> 00:00:02,000\nFirst\n").expect("write cues");
    let mkv = dir.join("mixed.mkv");
    let built = std::process::Command::new("ffmpeg")
        .args(["-nostdin", "-v", "error", "-y", "-i"])
        .arg(fixture("supsample.mkv"))
        .arg("-i")
        .arg(&cues)
        .args(["-map", "0:0", "-map", "0:1", "-map", "1:0", "-c:v", "copy"])
        .args(["-c:s:0", "dvbsub", "-c:s:1", "srt"])
        .arg(&mkv)
        .status()
        .expect("ffmpeg should build the fixture; is ffmpeg installed and on PATH?");
    assert!(built.success());
    mkv.to_string_lossy().into_owned()
}

#[tokio::test]
async fn a_track_that_cannot_become_text_is_left_out_of_the_read() {
    let source = tempfile::tempdir().expect("tempdir");
    let input = dvb_beside_srt(source.path());
    let root = tempfile::tempdir().expect("tempdir");
    let spawner = Scripted::default();

    let prep = preparer(root.path(), spawner.clone())
        .prepare(
            &version(),
            &input,
            &[track(1, SubtitleFormat::Srt), track(2, SubtitleFormat::Srt)],
        )
        .await
        .expect("prepare");

    assert_eq!(prep.extracted_subtitles, vec![2]);
    assert!(!prep.keyframes.is_empty());
    assert_eq!(spawner.runs(), 3, "a header probe, one read of the file, one conversion of a copy");
    assert!(!spawner.read().iter().any(|arg| arg == "0:1"));
    assert!(stored(root.path(), "embedded-1", "mks").is_empty());
    assert_eq!(stored(root.path(), "embedded-2", "vtt").len(), 1);
}

#[tokio::test]
async fn a_read_that_writes_no_listing_fails() {
    let root = tempfile::tempdir().expect("tempdir");
    let spawner = Scripted {
        instead: Some(|_| Some(Ok(CommandOutput { success: true, ..CommandOutput::default() }))),
        ..Scripted::default()
    };

    let result =
        preparer(root.path(), spawner).prepare(&version(), &fixture("sample_full.mkv"), &[]).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn a_version_id_that_is_not_a_plain_name_is_refused() {
    let root = tempfile::tempdir().expect("tempdir");
    let spawner = Scripted::default();

    let result = preparer(root.path(), spawner.clone())
        .prepare(&VersionId("../escape".to_owned()), &fixture("sample_full.mkv"), &[])
        .await;

    assert!(result.is_err());
    assert_eq!(spawner.runs(), 0);
}

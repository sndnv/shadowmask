use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use domain::catalog::VersionId;
use domain::media::{EmbeddedSubtitleTrack, MediaProbe, PlaybackPreparer, SubtitleFormat};
use domain::session::{
    BurnSubtitle, SegmentContainer, SessionId, StreamClaims, StreamGeneration, StreamSource,
    TranscodeManager, TranscodeSpec, TranscodeStarted,
};
use domain::user::UserId;
use jiff::Timestamp;
use media::hls::HlsStreamSource;
use media::playback_prep::FfmpegPlaybackPreparer;
use media::playback_prep_store::FsPlaybackPrepStore;
use media::probe::FfprobeMediaProbe;

const OUTPUT_OFFSET_S: f64 = 1.0;
const FRAME_TOLERANCE_S: f64 = 0.0015;
const LAST_SEGMENT_TOLERANCE_S: f64 = 0.25;
const HALF_FRAME_S: f64 = 0.021;
const GENERATION: StreamGeneration = StreamGeneration(1);

struct Clip {
    path: String,
    duration_ms: u64,
    keyframes_us: Vec<u64>,
}

async fn clip(dir: &Path, name: &str, seconds: u32, offset: &str, video: &[&str]) -> Clip {
    let path = dir.join(name).to_string_lossy().into_owned();
    let picture = format!("testsrc2=s=160x120:r=24000/1001:d={seconds}");
    let tone = format!("sine=d={seconds}");
    let built = Command::new("ffmpeg")
        .args(["-nostdin", "-v", "error", "-y", "-f", "lavfi", "-i", &picture])
        .args(["-f", "lavfi", "-i", &tone, "-map", "0", "-map", "1"])
        .args(["-c:v", "libx264", "-preset", "veryfast", "-pix_fmt", "yuv420p"])
        .args(video)
        .args(["-c:a", "aac", "-output_ts_offset", offset, &path])
        .status()
        .expect("ffmpeg should build the clip; is ffmpeg installed and on PATH?");
    assert!(built.success(), "building [{name}]");
    described(path, name).await
}

async fn described(path: String, name: &str) -> Clip {
    let duration_ms = FfprobeMediaProbe::default().probe(&path).await.expect("probe").duration_ms;
    let prep = tempfile::tempdir().expect("tempdir");
    let keyframes_us = FfmpegPlaybackPreparer::new(FsPlaybackPrepStore::new(prep.path()))
        .prepare(&VersionId(name.replace('.', "-")), &path, &[])
        .await
        .expect("prepare")
        .keyframes_us;
    Clip { path, duration_ms, keyframes_us }
}

fn fixture(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn spec(clip: &Clip, copy: bool, container: SegmentContainer, seek_ms: u64) -> TranscodeSpec {
    TranscodeSpec {
        session: SessionId("real".to_owned()),
        generation: GENERATION,
        version: VersionId("real".to_owned()),
        input_path: clip.path.clone(),
        duration_ms: clip.duration_ms,
        keyframes_us: Some(clip.keyframes_us.clone()),
        copy,
        container,
        seek_ms: (seek_ms > 0).then_some(seek_ms),
        audio_track: None,
        max_height: None,
        max_bitrate: None,
        burn_subtitle: None,
        soft_subtitle: None,
        convert_audio: false,
        audio_channels: None,
        source_hdr: None,
    }
}

fn claims() -> StreamClaims {
    StreamClaims {
        session: SessionId("real".to_owned()),
        generation: GENERATION,
        user: UserId("u1".to_owned()),
        version: VersionId("real".to_owned()),
        expires_at: Timestamp::now(),
        nonce: String::new(),
    }
}

fn engine(cache: &Path) -> HlsStreamSource {
    HlsStreamSource::new(cache, cache.join("_prep")).with_read_rate(0.0)
}

async fn run_to_the_end(
    cache: &Path,
    spec: TranscodeSpec,
) -> (HlsStreamSource, TranscodeStarted, PathBuf) {
    let engine = engine(cache);
    let (started, variant) = finish(&engine, spec).await;
    (engine, started, variant)
}

async fn finish(engine: &HlsStreamSource, spec: TranscodeSpec) -> (TranscodeStarted, PathBuf) {
    let started = engine.start(spec).await.expect("start");
    let variant = PathBuf::from(&started.output_dir).join("v0");
    for _ in 0..600 {
        let written = std::fs::read_to_string(variant.join("producer.m3u8")).unwrap_or_default();
        if written.contains("#EXT-X-ENDLIST") {
            return (started, variant);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("the run did not finish within a minute");
}

fn durations(playlist: &Path) -> Vec<f64> {
    std::fs::read_to_string(playlist)
        .expect("playlist")
        .lines()
        .filter_map(|line| line.strip_prefix("#EXTINF:"))
        .filter_map(|rest| rest.trim_end_matches(',').parse().ok())
        .collect()
}

fn assert_cut_as_planned(variant: &Path, tolerance_s: f64) {
    let planned = durations(&variant.join("index.m3u8"));
    let written = durations(&variant.join("producer.m3u8"));
    assert_eq!(planned.len(), written.len(), "planned {planned:?}, written {written:?}");
    let last = planned.len() - 1;
    for (index, (plan, made)) in planned.iter().zip(&written).enumerate() {
        let allowed = if index == last { LAST_SEGMENT_TOLERANCE_S } else { tolerance_s };
        assert!((plan - made).abs() <= allowed, "planned {planned:?}, written {written:?}");
    }
}

async fn assert_every_segment_served(engine: &HlsStreamSource, variant: &Path) {
    let listed = std::fs::read_to_string(variant.join("index.m3u8")).expect("playlist");
    for name in listed.lines().filter(|line| line.starts_with("seg_")) {
        let path = engine.media_path(&claims(), "v0", name).await.expect("served");
        assert!(path.is_file(), "{name}");
    }
}

fn video_times(input: &str) -> (Vec<f64>, String) {
    let listed = Command::new("ffprobe")
        .args(["-v", "warning", "-select_streams", "v:0", "-show_entries", "packet=pts_time"])
        .args(["-of", "csv=p=0", input])
        .output()
        .expect("ffprobe should run");
    let mut times: Vec<f64> = String::from_utf8_lossy(&listed.stdout)
        .lines()
        .filter_map(|line| line.trim().trim_end_matches(',').parse().ok())
        .collect();
    times.sort_by(f64::total_cmp);
    (times, String::from_utf8_lossy(&listed.stderr).into_owned())
}

fn start_time(input: &str) -> f64 {
    let listed = Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", "format=start_time", "-of", "csv=p=0", input])
        .output()
        .expect("ffprobe should run");
    String::from_utf8_lossy(&listed.stdout).trim().parse().unwrap_or(0.0)
}

fn assert_every_frame_once_from(clip: &Clip, variant: &Path, origin_ms: u64, tolerance_s: f64) {
    let start = start_time(&clip.path);
    let origin = origin_ms as f64 / 1000.0;
    let expected: Vec<f64> = video_times(&clip.path)
        .0
        .into_iter()
        .map(|time| time - start)
        .filter(|time| *time >= origin - FRAME_TOLERANCE_S)
        .map(|time| time + OUTPUT_OFFSET_S)
        .collect();
    let (served, warnings) = video_times(&variant.join("index.m3u8").to_string_lossy());
    assert!(warnings.trim().is_empty(), "{warnings}");
    assert_eq!(served.len(), expected.len(), "frames served against frames in the source");
    for (made, want) in served.iter().zip(&expected) {
        assert!((made - want).abs() <= tolerance_s, "frame at {made}, expected {want}");
    }
}

#[tokio::test]
async fn a_transcode_run_serves_every_frame_once_wherever_the_file_starts() {
    let dir = tempfile::tempdir().expect("tempdir");
    for (name, offset) in [("zero.mkv", "0"), ("late.ts", "1.46"), ("later.mp4", "9.977")] {
        let clip = clip(dir.path(), name, 20, offset, &["-g", "48"]).await;
        let cache = tempfile::tempdir().expect("tempdir");

        let (engine, started, variant) =
            run_to_the_end(cache.path(), spec(&clip, false, SegmentContainer::MpegTs, 0)).await;

        assert_eq!(started.origin_ms, clip.keyframes_us[0] / 1_000, "the first picture");
        assert_cut_as_planned(&variant, 0.05);
        assert_every_segment_served(&engine, &variant).await;
        assert_every_frame_once_from(&clip, &variant, started.origin_ms, HALF_FRAME_S);
    }
}

async fn sound_first(dir: &Path) -> Clip {
    let path = dir.join("sound-first.ts").to_string_lossy().into_owned();
    let built = Command::new("ffmpeg")
        .args(["-nostdin", "-v", "error", "-y", "-itsoffset", "0.3", "-f", "lavfi"])
        .args(["-i", "testsrc2=s=160x120:r=25:d=24", "-f", "lavfi", "-i", "sine=f=440:d=24.3"])
        .args(["-itsoffset", "0.5", "-f", "lavfi", "-i", "sine=f=660:d=23.8"])
        .args(["-map", "0", "-map", "1", "-map", "2", "-c:v", "libx264", "-preset", "veryfast"])
        .args(["-pix_fmt", "yuv420p", "-g", "100000", "-sc_threshold", "0"])
        .args(["-force_key_frames", "0,4,8,11.88,12.2,16,20", "-c:a", "aac", &path])
        .status()
        .expect("ffmpeg should build the clip; is ffmpeg installed and on PATH?");
    assert!(built.success());
    described(path, "sound-first.ts").await
}

#[tokio::test]
async fn a_copy_cuts_where_its_playlist_says_however_its_file_starts() {
    let dir = tempfile::tempdir().expect("tempdir");
    let later = clip(dir.path(), "later.mp4", 30, "9.977", &["-g", "48"]).await;
    let first = sound_first(dir.path()).await;
    for clip in [&later, &first] {
        for seek_ms in [0, 9_000] {
            let cache = tempfile::tempdir().expect("tempdir");
            let spec = TranscodeSpec {
                audio_track: Some(1),
                ..spec(clip, true, SegmentContainer::MpegTs, seek_ms)
            };

            let (engine, started, variant) = run_to_the_end(cache.path(), spec).await;

            let origin_us =
                clip.keyframes_us.iter().copied().filter(|k| *k <= seek_ms * 1_000).max();
            let origin_ms = origin_us.unwrap_or(clip.keyframes_us[0]) / 1_000;
            assert_eq!(started.origin_ms, origin_ms, "{} from {seek_ms}", clip.path);
            assert_cut_as_planned(&variant, 0.002);
            assert_every_segment_served(&engine, &variant).await;
            assert_every_frame_once_from(clip, &variant, origin_ms, FRAME_TOLERANCE_S);
        }
    }
}

#[tokio::test]
async fn a_transcode_starts_where_its_plan_does_whichever_sound_track_plays() {
    let dir = tempfile::tempdir().expect("tempdir");
    let clip = sound_first(dir.path()).await;
    for audio in [1, 2] {
        for seek_ms in [0, 8_000] {
            let cache = tempfile::tempdir().expect("tempdir");
            let spec = TranscodeSpec {
                audio_track: Some(audio),
                ..spec(&clip, false, SegmentContainer::MpegTs, seek_ms)
            };

            let (_engine, started, variant) = run_to_the_end(cache.path(), spec).await;

            let origin_ms = if seek_ms == 0 { clip.keyframes_us[0] / 1_000 } else { seek_ms };
            assert_eq!(started.origin_ms, origin_ms, "audio {audio} from {seek_ms}");
            assert_cut_as_planned(&variant, 0.05);
            assert_every_frame_once_from(&clip, &variant, origin_ms, HALF_FRAME_S);
        }
    }
}

async fn surround(dir: &Path, name: &str, offset: &str) -> Clip {
    let path = dir.join(name).to_string_lossy().into_owned();
    let built = Command::new("ffmpeg")
        .args(["-nostdin", "-v", "error", "-y", "-f", "lavfi"])
        .args(["-i", "testsrc2=s=160x120:r=24000/1001:d=30", "-f", "lavfi", "-i", "sine=d=30"])
        .args(["-map", "0", "-map", "1", "-c:v", "libx264", "-preset", "veryfast"])
        .args(["-pix_fmt", "yuv420p", "-g", "48", "-c:a", "ac3", "-ac", "6"])
        .args(["-output_ts_offset", offset, &path])
        .status()
        .expect("ffmpeg should build the clip; is ffmpeg installed and on PATH?");
    assert!(built.success(), "building [{name}]");
    described(path, name).await
}

fn span(input: &str, stream: &str) -> (f64, f64) {
    let listed = Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", stream])
        .args(["-show_entries", "packet=pts_time,duration_time", "-of", "csv=p=0", input])
        .output()
        .expect("ffprobe should run");
    String::from_utf8_lossy(&listed.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split(',').map(|field| field.trim().parse::<f64>());
            Some((fields.next()?.ok()?, fields.next()?.ok()?))
        })
        .fold((f64::MAX, f64::MIN), |(first, end), (at, length)| {
            (first.min(at), end.max(at + length))
        })
}

fn sound_format(input: &str) -> String {
    let listed = Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "a:0"])
        .args(["-show_entries", "stream=codec_name,channels", "-of", "csv=p=0", input])
        .output()
        .expect("ffprobe should run");
    let text = String::from_utf8_lossy(&listed.stdout).into_owned();
    text.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or_default().to_owned()
}

const AAC_LEAD_IN_S: f64 = 0.03;
const SOUND_END_TOLERANCE_S: f64 = 0.05;

#[tokio::test]
async fn a_copy_converting_its_sound_cuts_as_planned_and_plays_it_with_the_picture() {
    let dir = tempfile::tempdir().expect("tempdir");
    let zero = surround(dir.path(), "surround.mkv", "0").await;
    let later = surround(dir.path(), "surround.mp4", "9.977").await;
    let cases = [
        (&zero, SegmentContainer::MpegTs),
        (&later, SegmentContainer::MpegTs),
        (&zero, SegmentContainer::Fmp4),
    ];
    for (clip, container) in cases {
        for seek_ms in [0, 9_000] {
            let cache = tempfile::tempdir().expect("tempdir");
            let spec = TranscodeSpec {
                audio_track: Some(1),
                convert_audio: true,
                audio_channels: Some(2),
                ..spec(clip, true, container, seek_ms)
            };

            let (engine, started, variant) = run_to_the_end(cache.path(), spec).await;

            let case = format!("{} {container:?} from {seek_ms}", clip.path);
            let origin_us =
                clip.keyframes_us.iter().copied().filter(|k| *k <= seek_ms * 1_000).max();
            let origin_ms = origin_us.unwrap_or(clip.keyframes_us[0]) / 1_000;
            assert_eq!(started.origin_ms, origin_ms, "{case}");
            assert_cut_as_planned(&variant, 0.002);
            assert_every_segment_served(&engine, &variant).await;
            assert_every_frame_once_from(clip, &variant, origin_ms, FRAME_TOLERANCE_S);
            let served = variant.join("index.m3u8").to_string_lossy().into_owned();
            assert_eq!(sound_format(&served), "aac,2", "{case}");
            let (picture_from, picture_to) = span(&served, "v:0");
            let (sound_from, sound_to) = span(&served, "a:0");
            assert!(
                sound_from <= picture_from && picture_from - sound_from <= AAC_LEAD_IN_S,
                "{case}: sound from {sound_from}, picture from {picture_from}"
            );
            assert!(
                (sound_to - picture_to).abs() <= SOUND_END_TOLERANCE_S,
                "{case}: sound to {sound_to}, picture to {picture_to}"
            );
        }
    }
}

#[tokio::test]
async fn a_resumed_transcode_starts_on_the_grid_boundary_before_the_seek() {
    let dir = tempfile::tempdir().expect("tempdir");
    let clip = clip(dir.path(), "late.ts", 20, "1.46", &["-g", "48"]).await;
    let cache = tempfile::tempdir().expect("tempdir");

    let (_engine, started, variant) =
        run_to_the_end(cache.path(), spec(&clip, false, SegmentContainer::MpegTs, 9_000)).await;

    assert_eq!(started.origin_ms, 8_000);
    assert_cut_as_planned(&variant, 0.05);
    assert_every_frame_once_from(&clip, &variant, 8_000, HALF_FRAME_S);
}

#[tokio::test]
async fn a_seek_is_timed_from_where_the_file_starts_as_the_start_before_it_was() {
    let dir = tempfile::tempdir().expect("tempdir");
    let clip = clip(dir.path(), "late.ts", 20, "1.46", &["-g", "48"]).await;
    let cache = tempfile::tempdir().expect("tempdir");
    let engine = engine(cache.path());
    engine.start(spec(&clip, false, SegmentContainer::MpegTs, 0)).await.expect("start");
    let seek = TranscodeSpec {
        generation: StreamGeneration(2),
        ..spec(&clip, false, SegmentContainer::MpegTs, 9_000)
    };

    let (started, variant) = finish(&engine, seek).await;

    assert_eq!(started.origin_ms, 8_000);
    assert_cut_as_planned(&variant, 0.05);
    assert_every_frame_once_from(&clip, &variant, 8_000, HALF_FRAME_S);
}

#[tokio::test]
async fn a_resumed_copy_starts_on_its_keyframe_in_either_container() {
    let dir = tempfile::tempdir().expect("tempdir");
    let clip = clip(dir.path(), "copy.mkv", 20, "0", &["-g", "48"]).await;
    let origin_us = clip.keyframes_us.iter().copied().filter(|k| *k <= 9_000_000).max();
    let origin_ms = origin_us.expect("a keyframe before the seek") / 1_000;
    for container in [SegmentContainer::MpegTs, SegmentContainer::Fmp4] {
        let cache = tempfile::tempdir().expect("tempdir");

        let (engine, started, variant) =
            run_to_the_end(cache.path(), spec(&clip, true, container, 9_000)).await;

        assert_eq!(started.origin_ms, origin_ms, "the keyframe at or before the seek");
        assert_cut_as_planned(&variant, 0.002);
        assert_every_segment_served(&engine, &variant).await;
        assert_every_frame_once_from(&clip, &variant, origin_ms, FRAME_TOLERANCE_S);
    }
}

#[tokio::test]
async fn a_long_gop_copy_is_cut_where_its_playlist_says() {
    let dir = tempfile::tempdir().expect("tempdir");
    let gop = ["-g", "1000", "-keyint_min", "1000", "-sc_threshold", "0"];
    let forced = ["-force_key_frames", "0,9,10,11,20,21,30"];
    let video: Vec<&str> = gop.iter().chain(&forced).copied().collect();
    let clip = clip(dir.path(), "gop.mkv", 40, "0", &video).await;
    let cache = tempfile::tempdir().expect("tempdir");

    let (engine, _started, variant) =
        run_to_the_end(cache.path(), spec(&clip, true, SegmentContainer::MpegTs, 0)).await;

    assert_eq!(durations(&variant.join("index.m3u8")).len(), 6);
    assert_cut_as_planned(&variant, 0.002);
    assert_every_segment_served(&engine, &variant).await;
    assert_every_frame_once_from(&clip, &variant, 0, FRAME_TOLERANCE_S);
}

fn with_text_track(dir: &Path, clip: &Clip) -> String {
    let cues = dir.join("cues.srt");
    std::fs::write(&cues, "1\n00:00:01,000 --> 00:00:03,000\nHello\n").expect("write cues");
    let path = dir.join("subtitled.mkv");
    let built = Command::new("ffmpeg")
        .args(["-nostdin", "-v", "error", "-y", "-i", &clip.path, "-i"])
        .arg(&cues)
        .args(["-map", "0", "-map", "1", "-c", "copy", "-c:s", "srt"])
        .arg(&path)
        .status()
        .expect("ffmpeg should build the clip");
    assert!(built.success());
    path.to_string_lossy().into_owned()
}

#[tokio::test]
async fn a_run_with_no_audio_choice_leaves_the_film_subtitles_out() {
    let dir = tempfile::tempdir().expect("tempdir");
    let plain = clip(dir.path(), "plain.mkv", 12, "0", &["-g", "48"]).await;
    let subtitled = described(with_text_track(dir.path(), &plain), "subtitled.mkv").await;
    for copy in [true, false] {
        let cache = tempfile::tempdir().expect("tempdir");

        let (engine, _started, variant) =
            run_to_the_end(cache.path(), spec(&subtitled, copy, SegmentContainer::MpegTs, 0)).await;

        assert_every_segment_served(&engine, &variant).await;
        let stray: Vec<PathBuf> = std::fs::read_dir(&variant)
            .expect("variant")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "vtt"))
            .collect();
        assert!(stray.is_empty(), "copy [{copy}] wrote {stray:?}");
    }
}

const CAPTION: &str =
    "crop=400:40:160:430,signalstats,metadata=print:key=lavfi.signalstats.YAVG:file=-";
const CUE_TOLERANCE_S: f64 = 0.1;

fn white_film(dir: &Path, subtitles: &[&str], mapped: &str) -> String {
    let path = dir.join("film.mkv");
    let built = Command::new("ffmpeg")
        .args(["-nostdin", "-v", "error", "-y", "-f", "lavfi"])
        .args(["-i", "color=c=white:s=720x480:r=25:d=52"])
        .args(subtitles)
        .args(["-map", "0:v", "-map", mapped, "-c:v", "libx264", "-preset", "veryfast"])
        .args(["-pix_fmt", "yuv420p", "-g", "50", "-c:s", "copy"])
        .arg(&path)
        .status()
        .expect("ffmpeg should build the film");
    assert!(built.success());
    path.to_string_lossy().into_owned()
}

fn caption_spans(variant: &Path) -> Vec<(bool, f64)> {
    let listed = Command::new("ffmpeg")
        .args(["-v", "error", "-copyts", "-i"])
        .arg(variant.join("index.m3u8"))
        .args(["-vf", CAPTION, "-f", "null", "-"])
        .output()
        .expect("ffmpeg should read the stream");
    let mut spans: Vec<(bool, f64)> = Vec::new();
    let mut at = 0.0;
    for line in String::from_utf8_lossy(&listed.stdout).lines() {
        if let Some((_, time)) = line.split_once("pts_time:") {
            at = time.trim().parse::<f64>().expect("a frame time") - OUTPUT_OFFSET_S;
        }
        if let Some((_, value)) = line.split_once("YAVG=") {
            let drawn = value.trim().parse::<f64>().expect("a brightness") < 230.0;
            if spans.last().is_none_or(|(was, _)| *was != drawn) {
                spans.push((drawn, at));
            }
        }
    }
    spans
}

fn assert_spans(spans: &[(bool, f64)], expected: &[(bool, f64)]) {
    let shown: Vec<bool> = spans.iter().map(|(drawn, _)| *drawn).collect();
    let wanted: Vec<bool> = expected.iter().map(|(drawn, _)| *drawn).collect();
    assert_eq!(shown, wanted, "caption spans {spans:?}");
    for ((_, at), (_, want)) in spans.iter().zip(expected) {
        assert!(
            (at - want).abs() <= CUE_TOLERANCE_S,
            "caption spans {spans:?}, expected {expected:?}"
        );
    }
}

async fn burned_spans(
    film: &str,
    track: EmbeddedSubtitleTrack,
    burn: BurnSubtitle,
) -> Vec<Vec<(bool, f64)>> {
    let mut found = Vec::new();
    for seek_ms in [0, 44_000] {
        let cache = tempfile::tempdir().expect("tempdir");
        let prep =
            FfmpegPlaybackPreparer::new(FsPlaybackPrepStore::new(cache.path().join("_prep")))
                .prepare(&VersionId("real".to_owned()), film, std::slice::from_ref(&track))
                .await
                .expect("prepare");
        assert_eq!(prep.extracted_subtitles, vec![track.index]);
        let clip =
            Clip { path: film.to_owned(), duration_ms: 52_000, keyframes_us: prep.keyframes_us };
        let spec = TranscodeSpec {
            burn_subtitle: Some(burn.clone()),
            ..spec(&clip, false, SegmentContainer::MpegTs, seek_ms)
        };
        let (_engine, _started, variant) = run_to_the_end(cache.path(), spec).await;
        found.push(caption_spans(&variant));
    }
    found
}

fn subtitle_track(index: u32, format: SubtitleFormat) -> EmbeddedSubtitleTrack {
    EmbeddedSubtitleTrack { index, language: None, format, forced: false, default: false }
}

#[tokio::test]
async fn an_image_subtitle_is_burned_at_its_own_time_from_the_start_and_from_a_resume() {
    let dir = tempfile::tempdir().expect("tempdir");
    let pictures = fixture("supsample.mkv");
    let film = white_film(dir.path(), &["-itsoffset", "40", "-i", &pictures], "1:s");
    let burn = BurnSubtitle::EmbeddedImage { index: 1, width: 720, height: 480 };

    let spans = burned_spans(&film, subtitle_track(1, SubtitleFormat::Pgs), burn).await;

    assert_spans(&spans[0], &[(false, 0.0), (true, 41.001), (false, 49.009)]);
    assert_spans(&spans[1], &[(true, 44.0), (false, 49.009)]);
}

fn has_filter(name: &str) -> bool {
    let listed = Command::new("ffmpeg")
        .args(["-hide_banner", "-filters"])
        .output()
        .expect("ffmpeg should list its filters");
    String::from_utf8_lossy(&listed.stdout)
        .lines()
        .any(|line| line.split_whitespace().nth(1) == Some(name))
}

#[tokio::test]
async fn a_text_subtitle_is_burned_at_its_own_time_from_the_start_and_from_a_resume() {
    if !has_filter("subtitles") {
        eprintln!("skipped: this ffmpeg has no [subtitles] filter (it was built without libass)");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let cues = dir.path().join("cues.srt");
    std::fs::write(&cues, "1\n00:00:41,000 --> 00:00:49,000\nBURNED TEXT CHECK\n").expect("cues");
    let cues = cues.to_string_lossy().into_owned();
    let film = white_film(dir.path(), &["-i", &cues], "1");

    let spans =
        burned_spans(&film, subtitle_track(1, SubtitleFormat::Srt), BurnSubtitle::EmbeddedText(1))
            .await;

    assert_spans(&spans[0], &[(false, 0.0), (true, 41.0), (false, 49.0)]);
    assert_spans(&spans[1], &[(true, 44.0), (false, 49.0)]);
}

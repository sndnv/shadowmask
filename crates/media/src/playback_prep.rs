use std::path::{Path, PathBuf};
use std::time::Instant;

use domain::catalog::VersionId;
use domain::error::PlaybackPrepError;
use domain::media::{EmbeddedSubtitleTrack, PlaybackPrep, PlaybackPreparer, SubtitleFormat};
use domain::process::ProcessSpawner;
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;
use uuid::Uuid;

use crate::diagnostics::parse_version;
use crate::hls::{has_content, remove_file_off_lock, rename_off_lock};
use crate::playback_prep_store::{FsPlaybackPrepStore, PrepEntry, embedded_prefix};
use crate::transcode::{TokioProcessSpawner, parse_start_us};

const FFMPEG_BINARY: &str = "ffmpeg";
const PROBE_BINARY: &str = "ffprobe";
const KEYFRAMES_PREFIX: &str = "keyframes";
const FAILURE_LINES: usize = 10;

pub struct FfmpegPlaybackPreparer<S = TokioProcessSpawner> {
    spawner: S,
    store: FsPlaybackPrepStore,
    ffmpeg: OnceCell<String>,
}

impl FfmpegPlaybackPreparer {
    pub fn new(store: FsPlaybackPrepStore) -> Self {
        Self::with_spawner(store, TokioProcessSpawner)
    }
}

impl<S> FfmpegPlaybackPreparer<S> {
    pub fn with_spawner(store: FsPlaybackPrepStore, spawner: S) -> Self {
        Self { spawner, store, ffmpeg: OnceCell::new() }
    }
}

#[derive(Serialize, Deserialize)]
struct Stored {
    prepared_with: String,
    keyframes_us: Vec<u64>,
}

struct Output {
    entry: PrepEntry,
    part: PathBuf,
}

impl Output {
    fn new(entry: PrepEntry, run: &str) -> Self {
        let part = part_of(&entry.path, &format!("{run}.part"));
        Self { entry, part }
    }
}

struct Track {
    index: u32,
    format: SubtitleFormat,
    mks: Output,
    vtt: Option<Output>,
}

const TEXT_CODECS: [&str; 17] = [
    "subrip",
    "srt",
    "ass",
    "ssa",
    "webvtt",
    "mov_text",
    "text",
    "microdvd",
    "mpl2",
    "jacosub",
    "sami",
    "realtext",
    "subviewer",
    "subviewer1",
    "vplayer",
    "pjs",
    "stl",
];

#[derive(Default, Deserialize)]
struct Probed {
    #[serde(default)]
    streams: Vec<ProbedStream>,
    #[serde(default)]
    format: ProbedFormat,
}

#[derive(Deserialize)]
struct ProbedStream {
    index: u32,
    codec_name: Option<String>,
}

#[derive(Default, Deserialize)]
struct ProbedFormat {
    start_time: Option<String>,
}

impl Probed {
    fn parse(stdout: &str) -> Self {
        serde_json::from_str(stdout).unwrap_or_default()
    }

    fn start_us(&self) -> i64 {
        self.format.start_time.as_deref().map_or(0, parse_start_us)
    }

    fn codec(&self, index: u32) -> Option<&str> {
        self.streams
            .iter()
            .find(|stream| stream.index == index)
            .and_then(|stream| stream.codec_name.as_deref())
    }
}

impl<S> PlaybackPreparer for FfmpegPlaybackPreparer<S>
where
    S: ProcessSpawner + Send + Sync,
{
    async fn prepare(
        &self,
        version: &VersionId,
        input_path: &str,
        subtitles: &[EmbeddedSubtitleTrack],
    ) -> Result<PlaybackPrep, PlaybackPrepError> {
        let run = Uuid::new_v4().simple().to_string();
        let keyframes = Output::new(
            self.store.output(version, input_path, KEYFRAMES_PREFIX, "json").await.map_err(fail)?,
            &run,
        );
        let probed = self.probe(input_path).await.map_err(fail)?;
        let mut tracks = Vec::with_capacity(subtitles.len());
        for track in subtitles {
            let codec = probed.codec(track.index);
            if is_text(track.format) && !is_text_codec(codec) {
                #[rustfmt::skip]
                tracing::warn!("version [{}] subtitle track [{}] ({}) cannot be converted to text, so it is left out", version.0, track.index, codec.unwrap_or("unknown"));
                continue;
            }
            let prefix = embedded_prefix(track.index);
            let mks = self.store.output(version, input_path, &prefix, "mks").await.map_err(fail)?;
            let vtt = match is_text(track.format) {
                true => Some(Output::new(
                    self.store.output(version, input_path, &prefix, "vtt").await.map_err(fail)?,
                    &run,
                )),
                false => None,
            };
            let mks = Output::new(mks, &run);
            tracks.push(Track { index: track.index, format: track.format, mks, vtt });
        }
        let ffmpeg = self.ffmpeg_version().await?.to_owned();
        if let Some(prep) = cached(&keyframes.entry, &tracks, &ffmpeg).await {
            tracing::info!("version [{}] is already prepared for playback", version.0);
            return Ok(prep);
        }
        tokio::fs::create_dir_all(&keyframes.entry.dir).await.map_err(fail)?;
        let started = Instant::now();
        let crc = part_of(&keyframes.entry.path, &format!("{run}.crc"));
        let copied = match self.read_once(input_path, &crc, &tracks).await {
            Ok(()) => true,
            Err(detail) if !tracks.is_empty() => {
                #[rustfmt::skip]
                tracing::warn!("could not copy the subtitle tracks of version [{}], so only its keyframes are kept: {detail}", version.0);
                self.read_once(input_path, &crc, &[]).await.map_err(fail)?;
                false
            }
            Err(detail) => return Err(fail(detail)),
        };
        let frames = tokio::fs::read_to_string(&crc).await.map_err(fail);
        remove_file_off_lock(crc).await;
        let points = on_film_clock(&parse_framecrc(&frames?)?, probed.start_us());
        let stored = Stored { prepared_with: ffmpeg, keyframes_us: points };
        self.keep_keyframes(&keyframes, &stored).await?;
        let mut extracted = Vec::new();
        if copied {
            for track in &tracks {
                if self.keep_track(version, track).await {
                    extracted.push(track.index);
                }
            }
        }
        #[rustfmt::skip]
        tracing::info!("prepared version [{}] for playback in [{:.1}s]: [{}] keyframes, [{}] of [{}] subtitle tracks", version.0, started.elapsed().as_secs_f64(), stored.keyframes_us.len(), extracted.len(), tracks.len());
        Ok(PlaybackPrep {
            keyframes_us: stored.keyframes_us,
            extracted_subtitles: extracted,
            prepared_with: stored.prepared_with,
        })
    }
}

impl<S> FfmpegPlaybackPreparer<S>
where
    S: ProcessSpawner + Send + Sync,
{
    async fn ffmpeg_version(&self) -> Result<&str, PlaybackPrepError> {
        let version = self
            .ffmpeg
            .get_or_try_init(|| async {
                let args = ["-hide_banner".to_owned(), "-version".to_owned()];
                match self.spawner.run_captured(FFMPEG_BINARY, &args).await {
                    Ok(output) if output.success => {
                        Ok(parse_version(&output.stdout).unwrap_or_default())
                    }
                    Ok(output) => Err(output.failure_detail(FAILURE_LINES)),
                    Err(err) => Err(err.to_string()),
                }
            })
            .await
            .map_err(|detail| fail(format!("could not read the ffmpeg version: {detail}")))?;
        Ok(version)
    }

    async fn probe(&self, input: &str) -> Result<Probed, String> {
        let outcome = self
            .spawner
            .run_captured(PROBE_BINARY, &probe_args(input))
            .await
            .map_err(|e| e.to_string())?;
        match outcome.success {
            true => Ok(Probed::parse(&outcome.stdout)),
            false => Err(outcome.failure_detail(FAILURE_LINES)),
        }
    }

    async fn read_once(&self, input: &str, crc: &Path, tracks: &[Track]) -> Result<(), String> {
        let args = read_args(input, crc, tracks);
        let outcome = self.spawner.run_captured(FFMPEG_BINARY, &args).await;
        let failure = match outcome {
            Ok(outcome) if outcome.success => return Ok(()),
            Ok(outcome) => outcome.failure_detail(FAILURE_LINES),
            Err(err) => err.to_string(),
        };
        for track in tracks {
            remove_file_off_lock(track.mks.part.clone()).await;
        }
        remove_file_off_lock(crc.to_path_buf()).await;
        Err(failure)
    }

    async fn keep_keyframes(
        &self,
        output: &Output,
        stored: &Stored,
    ) -> Result<(), PlaybackPrepError> {
        let encoded = serde_json::to_vec(stored).map_err(fail)?;
        tokio::fs::write(&output.part, encoded).await.map_err(fail)?;
        keep(output.part.clone(), &output.entry.path).await.map_err(fail)?;
        self.store.prune_siblings(&output.entry).await;
        Ok(())
    }

    async fn keep_track(&self, version: &VersionId, track: &Track) -> bool {
        match self.try_keep_track(track).await {
            Ok(()) => true,
            Err(detail) => {
                #[rustfmt::skip]
                tracing::warn!("version [{}] subtitle track [{}] ({:?}) was not kept: {detail}", version.0, track.index, track.format);
                false
            }
        }
    }

    async fn try_keep_track(&self, track: &Track) -> Result<(), String> {
        keep(track.mks.part.clone(), &track.mks.entry.path).await?;
        self.store.prune_siblings(&track.mks.entry).await;
        let Some(vtt) = &track.vtt else {
            return Ok(());
        };
        let args = vtt_args(&track.mks.entry.path, &vtt.part);
        let outcome =
            self.spawner.run_captured(FFMPEG_BINARY, &args).await.map_err(|e| e.to_string())?;
        if !outcome.success || !has_content(&vtt.part).await {
            remove_file_off_lock(vtt.part.clone()).await;
            return Err(outcome.failure_detail(FAILURE_LINES));
        }
        keep(vtt.part.clone(), &vtt.entry.path).await?;
        self.store.prune_siblings(&vtt.entry).await;
        Ok(())
    }
}

async fn keep(part: PathBuf, path: &Path) -> Result<(), String> {
    match rename_off_lock(part, path.to_path_buf()).await {
        true => Ok(()),
        false => Err(format!("[{}] could not be kept", path.display())),
    }
}

async fn cached(keyframes: &PrepEntry, tracks: &[Track], ffmpeg: &str) -> Option<PlaybackPrep> {
    for track in tracks {
        if !has_content(&track.mks.entry.path).await {
            return None;
        }
        if let Some(vtt) = &track.vtt
            && !has_content(&vtt.entry.path).await
        {
            return None;
        }
    }
    let stored = tokio::fs::read(&keyframes.path).await.ok()?;
    let stored: Stored = serde_json::from_slice(&stored).ok()?;
    (stored.prepared_with == ffmpeg).then(|| PlaybackPrep {
        keyframes_us: stored.keyframes_us,
        extracted_subtitles: tracks.iter().map(|track| track.index).collect(),
        prepared_with: stored.prepared_with,
    })
}

fn is_text(format: SubtitleFormat) -> bool {
    matches!(format, SubtitleFormat::Srt | SubtitleFormat::Ass | SubtitleFormat::Vtt)
}

fn codec_for(format: SubtitleFormat) -> &'static str {
    match format {
        SubtitleFormat::Srt | SubtitleFormat::Vtt => "srt",
        SubtitleFormat::Ass | SubtitleFormat::Pgs | SubtitleFormat::VobSub => "copy",
    }
}

fn part_of(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{suffix}"));
    path.with_file_name(name)
}

fn read_args(input: &str, crc: &Path, tracks: &[Track]) -> Vec<String> {
    let mut args: Vec<String> = ["-nostdin", "-v", "error", "-y", "-copyts", "-i", input]
        .iter()
        .map(|arg| (*arg).to_owned())
        .collect();
    args.extend(["-map", "0:v:0", "-c", "copy", "-f", "framecrc"].map(str::to_owned));
    args.push(crc.to_string_lossy().into_owned());
    for track in tracks {
        args.push("-map".to_owned());
        args.push(format!("0:{}", track.index));
        if matches!(track.format, SubtitleFormat::Ass) {
            args.extend(["-map", "0:t?", "-c:t", "copy"].map(str::to_owned));
        }
        args.push("-c:s".to_owned());
        args.push(codec_for(track.format).to_owned());
        args.push("-f".to_owned());
        args.push("matroska".to_owned());
        args.push(track.mks.part.to_string_lossy().into_owned());
    }
    args
}

fn on_film_clock(keyframes_on_file: &[u64], start_us: i64) -> Vec<u64> {
    keyframes_on_file
        .iter()
        .filter_map(|&keyframe| {
            let film = i64::try_from(keyframe).ok()?.checked_sub(start_us)?;
            u64::try_from(film).ok()
        })
        .collect()
}

fn is_text_codec(codec: Option<&str>) -> bool {
    codec.is_some_and(|codec| TEXT_CODECS.contains(&codec))
}

fn probe_args(input: &str) -> Vec<String> {
    let entries = "stream=index,codec_name:format=start_time";
    ["-v", "error", "-show_entries", entries, "-select_streams", "s", "-of", "json", input]
        .map(str::to_owned)
        .to_vec()
}

fn vtt_args(mks: &Path, out: &Path) -> Vec<String> {
    let mks = mks.to_string_lossy();
    let out = out.to_string_lossy();
    ["-nostdin", "-v", "error", "-y", "-i", &*mks, "-f", "webvtt", &*out]
        .map(str::to_owned)
        .to_vec()
}

fn parse_framecrc(text: &str) -> Result<Vec<u64>, PlaybackPrepError> {
    let mut time_base = None;
    let mut keyframes = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("#tb 0:") {
            time_base = parse_time_base(rest.trim());
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        let (Some((num, den)), Some(pts)) = (time_base, fields.get(2)) else {
            continue;
        };
        let Ok(pts) = pts.parse::<i64>() else {
            continue;
        };
        if pts < 0 || !is_keyframe(&fields) {
            continue;
        }
        let scaled = u128::from(pts.unsigned_abs()) * u128::from(num) * 1_000_000;
        keyframes.push(((scaled + u128::from(den) / 2) / u128::from(den)) as u64);
    }
    if time_base.is_none() {
        return Err(fail("ffmpeg listed no video time base"));
    }
    keyframes.sort_unstable();
    keyframes.dedup();
    Ok(keyframes)
}

fn parse_time_base(text: &str) -> Option<(u64, u64)> {
    let (num, den) = text.split_once('/')?;
    let num = num.trim().parse::<u64>().ok()?;
    let den = den.trim().parse::<u64>().ok()?;
    (num > 0 && den > 0).then_some((num, den))
}

fn is_keyframe(fields: &[&str]) -> bool {
    match fields.iter().find_map(|field| field.strip_prefix("F=0x")) {
        None => true,
        Some(hex) => u32::from_str_radix(hex, 16).is_ok_and(|flags| flags & 1 == 1),
    }
}

fn fail(detail: impl std::fmt::Display) -> PlaybackPrepError {
    PlaybackPrepError::Backend(detail.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTING: &str = "#tb 0: 1/1000\n\
        #media_type 0: video\n\
        0,        -83,          0,       41,     2345, 0x4a3d21d7\n\
        0,        -42,         83,       41,      412, 0x11111111, F=0x0\n\
        0,          0,       4000,       41,     2345, 0x22222222, S=1, 8\n\
        0,         41,       8000,       41,     2345, 0x33333333, F=0x5\n\
        0,         83,         42,       41,      300, 0x44444444, F=0x0\n";

    #[test]
    fn a_packet_is_a_keyframe_unless_its_flags_say_otherwise() {
        assert_eq!(parse_framecrc(LISTING).unwrap(), vec![0, 4_000_000, 8_000_000]);
    }

    #[test]
    fn timestamps_are_converted_from_the_stream_time_base_to_microseconds() {
        let listing = "#tb 0: 1/15360\n0, 0, 0, 512, 10, 0x0\n0, 1, 128000, 512, 10, 0x0\n";
        assert_eq!(parse_framecrc(listing).unwrap(), vec![0, 8_333_333]);
        let mpegts = "#tb 0: 1/90000\n0, 0, 900900, 3003, 10, 0x0\n";
        assert_eq!(parse_framecrc(mpegts).unwrap(), vec![10_010_000]);
    }

    #[test]
    fn a_packet_before_the_start_or_without_a_timestamp_is_not_a_keyframe() {
        let listing =
            "#tb 0: 1/1000\n0, -10, -10, 1, 1, 0x0\n0, 0, x, 1, 1, 0x0\n0, 5, 5, 1, 1, 0x0\n";
        assert_eq!(parse_framecrc(listing).unwrap(), vec![5_000]);
    }

    #[test]
    fn a_listing_without_a_time_base_is_refused() {
        assert!(parse_framecrc("0, 0, 0, 1, 1, 0x0\n").is_err());
        assert!(parse_framecrc("#tb 0: 0/1000\n0, 0, 0, 1, 1, 0x0\n").is_err());
        assert!(parse_framecrc("#tb 0: 1\n0, 0, 0, 1, 1, 0x0\n").is_err());
    }

    #[test]
    fn text_tracks_are_kept_as_text_and_image_tracks_are_copied() {
        assert_eq!(codec_for(SubtitleFormat::Srt), "srt");
        assert_eq!(codec_for(SubtitleFormat::Vtt), "srt");
        assert_eq!(codec_for(SubtitleFormat::Ass), "copy");
        assert_eq!(codec_for(SubtitleFormat::Pgs), "copy");
        assert_eq!(codec_for(SubtitleFormat::VobSub), "copy");
        assert!(is_text(SubtitleFormat::Ass));
        assert!(!is_text(SubtitleFormat::Pgs));
    }

    #[test]
    fn the_read_keeps_the_timestamps_of_the_file() {
        let args = read_args("/x/movie.mkv", Path::new("/x/keyframes.crc"), &[]);
        let at = args.iter().position(|arg| arg == "-i").unwrap();
        assert_eq!(args[at - 1..=at + 1], ["-copyts", "-i", "/x/movie.mkv"]);
        assert!(!args.iter().any(|arg| arg == "-start_at_zero" || arg == "-itsoffset"));
    }

    fn track_to(dir: &Path, index: u32, format: SubtitleFormat) -> Track {
        let entry = PrepEntry {
            dir: dir.to_path_buf(),
            path: dir.join(format!("embedded-{index}.mks")),
            prefix: format!("embedded-{index}"),
            extension: "mks".to_owned(),
        };
        Track { index, format, mks: Output::new(entry, "run"), vtt: None }
    }

    #[test]
    fn an_ass_copy_carries_the_fonts_of_the_film_and_other_copies_do_not() {
        let dir = Path::new("/x");
        let tracks = [track_to(dir, 2, SubtitleFormat::Ass), track_to(dir, 3, SubtitleFormat::Srt)];
        let args = read_args("/x/movie.mkv", Path::new("/x/keyframes.crc"), &tracks);
        let output_of = |index: u32| -> Vec<String> {
            let from = args.iter().position(|arg| *arg == format!("0:{index}")).unwrap();
            let to = from + args[from..].iter().position(|arg| arg.ends_with(".part")).unwrap();
            args[from..to].to_vec()
        };
        assert!(output_of(2).windows(4).any(|w| w == ["-map", "0:t?", "-c:t", "copy"]));
        assert!(!output_of(3).iter().any(|arg| arg == "0:t?"));
    }

    #[test]
    fn the_probe_asks_for_the_subtitle_codecs_and_where_the_file_starts() {
        let args = probe_args("/x/movie.mkv");
        let at = args.iter().position(|arg| arg == "-show_entries").unwrap();
        assert_eq!(args[at + 1], "stream=index,codec_name:format=start_time");
    }

    #[test]
    fn the_start_of_the_file_comes_from_the_probe() {
        let probed = Probed::parse(r#"{"streams": [], "format": {"start_time": "1.400000"}}"#);
        assert_eq!(probed.start_us(), 1_400_000);
        assert_eq!(Probed::parse(r#"{"format": {}}"#).start_us(), 0);
        assert_eq!(Probed::parse("not json").start_us(), 0);
    }

    #[test]
    fn keyframes_move_from_the_file_clock_to_the_film_clock() {
        assert_eq!(on_film_clock(&[1_743_222, 5_423_222], 1_400_000), vec![343_222, 4_023_222]);
        assert_eq!(on_film_clock(&[23_000, 2_023_000], -23_220), vec![46_220, 2_046_220]);
        assert_eq!(on_film_clock(&[1_000_000, 2_000_000], 1_500_000), vec![500_000]);
    }

    #[test]
    fn a_track_is_converted_only_when_the_probe_lists_a_text_codec_for_it() {
        let probed = Probed::parse(
            r#"{"streams": [
                {"index": 1, "codec_name": "dvb_subtitle"},
                {"index": 2, "codec_name": "subrip"},
                {"index": 3, "codec_name": "mov_text"},
                {"index": 4}
            ]}"#,
        );
        assert!(!is_text_codec(probed.codec(1)));
        assert!(is_text_codec(probed.codec(2)));
        assert!(is_text_codec(probed.codec(3)));
        assert!(!is_text_codec(probed.codec(4)));
        assert!(!is_text_codec(probed.codec(5)));
        assert!(!is_text_codec(Probed::parse("not json").codec(2)));
    }

    #[test]
    fn cues_are_converted_on_the_timeline_of_their_copy() {
        let args = vtt_args(Path::new("/x/track.mks"), Path::new("/x/track.vtt"));
        assert_eq!(
            args,
            ["-nostdin", "-v", "error", "-y", "-i", "/x/track.mks", "-f", "webvtt", "/x/track.vtt"]
        );
    }
}

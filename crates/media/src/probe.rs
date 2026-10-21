use std::collections::HashMap;

use serde::Deserialize;
use tokio::process::Command;

use domain::common::LanguageCode;
use domain::error::ProbeError;
use domain::media::{
    AudioTrack, Chapter, EmbeddedSubtitleTrack, HdrFormat, MediaProbe, ProbeResult, SubtitleFormat,
    VideoTrack,
};

#[derive(Debug, Clone)]
pub struct FfprobeMediaProbe {
    binary: String,
}

impl Default for FfprobeMediaProbe {
    fn default() -> Self {
        Self { binary: "ffprobe".to_owned() }
    }
}

impl FfprobeMediaProbe {
    pub fn with_binary(binary: impl Into<String>) -> Self {
        Self { binary: binary.into() }
    }
}

impl MediaProbe for FfprobeMediaProbe {
    async fn probe(&self, path: &str) -> Result<ProbeResult, ProbeError> {
        let output = Command::new(&self.binary)
            .args([
                "-v",
                "error",
                "-print_format",
                "json",
                "-show_format",
                "-show_streams",
                "-show_chapters",
                "-show_pixel_formats",
            ])
            .arg(path)
            .output()
            .await
            .map_err(|e| ProbeError::Backend(e.to_string()))?;
        if !output.status.success() {
            return Err(ProbeError::Backend(String::from_utf8_lossy(&output.stderr).into_owned()));
        }
        parse_probe(&output.stdout)
    }
}

#[derive(Deserialize)]
struct FfOutput {
    #[serde(default)]
    format: FfFormat,
    #[serde(default)]
    streams: Vec<FfStream>,
    #[serde(default)]
    chapters: Vec<FfChapter>,
    #[serde(default)]
    pixel_formats: Vec<FfPixelFormat>,
}

#[derive(Deserialize)]
struct FfPixelFormat {
    name: Option<String>,
    #[serde(default)]
    components: Vec<FfComponent>,
}

#[derive(Deserialize)]
struct FfComponent {
    bit_depth: Option<u8>,
}

#[derive(Default, Deserialize)]
struct FfFormat {
    duration: Option<String>,
}

#[derive(Deserialize)]
struct FfStream {
    index: u32,
    codec_type: String,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    channels: Option<u8>,
    bit_rate: Option<String>,
    bits_per_raw_sample: Option<String>,
    pix_fmt: Option<String>,
    r_frame_rate: Option<String>,
    color_transfer: Option<String>,
    #[serde(default)]
    side_data_list: Vec<FfSideData>,
    #[serde(default)]
    disposition: FfDisposition,
    #[serde(default)]
    tags: FfTags,
}

#[derive(Deserialize)]
struct FfSideData {
    side_data_type: Option<String>,
}

#[derive(Default, Deserialize)]
struct FfDisposition {
    #[serde(default)]
    default: u8,
    #[serde(default)]
    forced: u8,
}

#[derive(Default, Deserialize)]
struct FfTags {
    language: Option<String>,
    title: Option<String>,
}

#[derive(Deserialize)]
struct FfChapter {
    start_time: Option<String>,
    #[serde(default)]
    tags: FfTags,
}

fn parse_probe(json: &[u8]) -> Result<ProbeResult, ProbeError> {
    let parsed: FfOutput =
        serde_json::from_slice(json).map_err(|e| ProbeError::Parse(e.to_string()))?;

    let duration_ms = seconds_to_ms(parsed.format.duration.as_deref());
    let depths = pixel_depths(parsed.pixel_formats);

    let mut video = Vec::new();
    let mut audio = Vec::new();
    let mut subtitles = Vec::new();
    for stream in parsed.streams {
        match stream.codec_type.as_str() {
            "video" => video.push(video_track(stream, &depths)),
            "audio" => audio.push(audio_track(stream)),
            "subtitle" => subtitles.push(subtitle_track(stream)),
            _ => {}
        }
    }

    let chapters = parsed.chapters.into_iter().map(chapter).collect();

    Ok(ProbeResult { duration_ms, video, audio, subtitles, chapters })
}

fn pixel_depths(formats: Vec<FfPixelFormat>) -> HashMap<String, u8> {
    formats
        .into_iter()
        .filter_map(|f| Some((f.name?, f.components.iter().filter_map(|c| c.bit_depth).max()?)))
        .collect()
}

fn bit_depth(s: &FfStream, depths: &HashMap<String, u8>) -> u8 {
    s.bits_per_raw_sample
        .as_deref()
        .and_then(|b| b.parse().ok())
        .or_else(|| s.pix_fmt.as_deref().and_then(|p| depths.get(p).copied()))
        .unwrap_or(8)
}

fn video_track(s: FfStream, depths: &HashMap<String, u8>) -> VideoTrack {
    VideoTrack {
        index: s.index,
        bit_depth: bit_depth(&s, depths),
        codec: s.codec_name.unwrap_or_default(),
        width: s.width.unwrap_or(0),
        height: s.height.unwrap_or(0),
        hdr: hdr_format(s.color_transfer.as_deref(), &s.side_data_list),
        frame_rate: parse_frame_rate(s.r_frame_rate.as_deref()),
        bitrate: parse_bitrate(s.bit_rate.as_deref()),
    }
}

fn audio_track(s: FfStream) -> AudioTrack {
    AudioTrack {
        index: s.index,
        codec: s.codec_name.unwrap_or_default(),
        channels: s.channels.unwrap_or(0),
        language: s.tags.language.as_deref().map(LanguageCode::canonical),
        bitrate: parse_bitrate(s.bit_rate.as_deref()),
        title: track_title(&s.tags),
    }
}

fn subtitle_track(s: FfStream) -> EmbeddedSubtitleTrack {
    EmbeddedSubtitleTrack {
        index: s.index,
        language: s.tags.language.as_deref().map(LanguageCode::canonical),
        format: subtitle_format(s.codec_name.as_deref()),
        forced: s.disposition.forced == 1,
        default: s.disposition.default == 1,
        title: track_title(&s.tags),
    }
}

fn track_title(tags: &FfTags) -> Option<String> {
    tags.title.as_deref().map(str::trim).filter(|title| !title.is_empty()).map(str::to_owned)
}

fn chapter(c: FfChapter) -> Chapter {
    Chapter {
        title: c.tags.title.unwrap_or_default(),
        start_ms: seconds_to_ms(c.start_time.as_deref()),
    }
}

fn seconds_to_ms(raw: Option<&str>) -> u64 {
    raw.and_then(|s| s.parse::<f64>().ok()).map(|secs| (secs * 1000.0) as u64).unwrap_or(0)
}

fn parse_bitrate(raw: Option<&str>) -> Option<u64> {
    raw.and_then(|b| b.parse().ok())
}

fn parse_frame_rate(raw: Option<&str>) -> f32 {
    let Some(raw) = raw else { return 0.0 };
    let mut parts = raw.split('/');
    let num: f32 = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0.0);
    let den: f32 = parts.next().and_then(|d| d.parse().ok()).unwrap_or(1.0);
    if den == 0.0 { 0.0 } else { num / den }
}

fn subtitle_format(codec: Option<&str>) -> SubtitleFormat {
    match codec {
        Some("ass" | "ssa") => SubtitleFormat::Ass,
        Some("webvtt") => SubtitleFormat::Vtt,
        Some("hdmv_pgs_subtitle") => SubtitleFormat::Pgs,
        Some("dvd_subtitle") => SubtitleFormat::VobSub,
        Some("dvb_subtitle") => SubtitleFormat::Dvb,
        _ => SubtitleFormat::Srt,
    }
}

fn hdr_format(transfer: Option<&str>, side_data: &[FfSideData]) -> Option<HdrFormat> {
    let has = |needle: &str| {
        side_data.iter().filter_map(|s| s.side_data_type.as_deref()).any(|t| t.contains(needle))
    };
    if has("Dolby Vision") {
        return Some(HdrFormat::DolbyVision);
    }
    if has("HDR Dynamic Metadata") {
        return Some(HdrFormat::Hdr10Plus);
    }
    match transfer {
        Some("smpte2084") => Some(HdrFormat::Hdr10),
        Some("arib-std-b67") => Some(HdrFormat::Hlg),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] = include_bytes!("../tests/fixtures/probe_movie.json");

    fn parse(json: &[u8]) -> ProbeResult {
        parse_probe(json).expect("fixture should parse")
    }

    #[test]
    fn parses_fixture_duration_and_counts() {
        let r = parse(FIXTURE);
        assert_eq!(r.duration_ms, 5_400_500);
        assert_eq!(r.video.len(), 1);
        assert_eq!(r.audio.len(), 2);
        assert_eq!(r.subtitles.len(), 2);
        assert_eq!(r.chapters.len(), 2);
    }

    #[test]
    fn parses_video_track() {
        let v = &parse(FIXTURE).video[0];
        assert_eq!(v.index, 0);
        assert_eq!(v.codec, "hevc");
        assert_eq!(v.width, 3840);
        assert_eq!(v.height, 2160);
        assert_eq!(v.bit_depth, 10);
        assert_eq!(v.hdr, Some(HdrFormat::Hdr10));
        assert_eq!(v.bitrate, Some(45_000_000));
        assert!((v.frame_rate - 23.976).abs() < 0.001);
    }

    #[test]
    fn parses_audio_tracks_with_and_without_optional_fields() {
        let a = parse(FIXTURE).audio;
        assert_eq!(a[0].codec, "eac3");
        assert_eq!(a[0].channels, 6);
        assert_eq!(a[0].language, Some(LanguageCode("en".to_owned())));
        assert_eq!(a[0].bitrate, Some(768_000));
        assert_eq!(a[1].codec, "aac");
        assert_eq!(a[1].channels, 2);
        assert_eq!(a[1].language, None);
        assert_eq!(a[1].bitrate, None);
    }

    #[test]
    fn parses_subtitle_tracks_with_dispositions() {
        let s = parse(FIXTURE).subtitles;
        assert_eq!(s[0].format, SubtitleFormat::Srt);
        assert_eq!(s[0].language, Some(LanguageCode("en".to_owned())));
        assert!(s[0].default);
        assert!(!s[0].forced);
        assert_eq!(s[1].format, SubtitleFormat::Pgs);
        assert_eq!(s[1].language, Some(LanguageCode("es".to_owned())));
        assert!(!s[1].default);
        assert!(s[1].forced);
    }

    #[test]
    fn a_track_keeps_its_own_title_and_a_blank_one_is_none() {
        let r = parse(
            br#"{"format": {"duration": "10.0"}, "streams": [
                {"index": 1, "codec_type": "audio", "codec_name": "ac3", "tags": {"title": "Commentary"}},
                {"index": 2, "codec_type": "audio", "codec_name": "aac", "tags": {"title": "  "}},
                {"index": 3, "codec_type": "subtitle", "codec_name": "subrip", "tags": {"title": " SDH "}},
                {"index": 4, "codec_type": "subtitle", "codec_name": "subrip"}
            ]}"#,
        );
        assert_eq!(r.audio[0].title.as_deref(), Some("Commentary"));
        assert_eq!(r.audio[1].title, None);
        assert_eq!(r.subtitles[0].title.as_deref(), Some("SDH"));
        assert_eq!(r.subtitles[1].title, None);
    }

    #[test]
    fn parses_chapters() {
        let c = parse(FIXTURE).chapters;
        assert_eq!(c[0].title, "Opening");
        assert_eq!(c[0].start_ms, 0);
        assert_eq!(c[1].title, "The Middle");
        assert_eq!(c[1].start_ms, 1_800_000);
    }

    #[test]
    fn subtitle_format_mapping() {
        assert_eq!(subtitle_format(Some("ass")), SubtitleFormat::Ass);
        assert_eq!(subtitle_format(Some("ssa")), SubtitleFormat::Ass);
        assert_eq!(subtitle_format(Some("webvtt")), SubtitleFormat::Vtt);
        assert_eq!(subtitle_format(Some("hdmv_pgs_subtitle")), SubtitleFormat::Pgs);
        assert_eq!(subtitle_format(Some("dvd_subtitle")), SubtitleFormat::VobSub);
        assert_eq!(subtitle_format(Some("dvb_subtitle")), SubtitleFormat::Dvb);
        assert_eq!(subtitle_format(Some("subrip")), SubtitleFormat::Srt);
        assert_eq!(subtitle_format(None), SubtitleFormat::Srt);
    }

    fn hdr_of(json: &[u8]) -> Option<HdrFormat> {
        parse(json).video[0].hdr
    }

    #[test]
    fn hdr_detection() {
        let dolby = br#"{"streams":[{"index":0,"codec_type":"video","color_transfer":"smpte2084","side_data_list":[{"side_data_type":"Dolby Vision Configuration Record"}]}]}"#;
        assert_eq!(hdr_of(dolby), Some(HdrFormat::DolbyVision));

        let hdr10plus = br#"{"streams":[{"index":0,"codec_type":"video","color_transfer":"smpte2084","side_data_list":[{"side_data_type":"HDR Dynamic Metadata SMPTE2094-40 (HDR10+)"}]}]}"#;
        assert_eq!(hdr_of(hdr10plus), Some(HdrFormat::Hdr10Plus));

        let hlg =
            br#"{"streams":[{"index":0,"codec_type":"video","color_transfer":"arib-std-b67"}]}"#;
        assert_eq!(hdr_of(hlg), Some(HdrFormat::Hlg));

        let sdr = br#"{"streams":[{"index":0,"codec_type":"video","color_transfer":"bt709"}]}"#;
        assert_eq!(hdr_of(sdr), None);
    }

    #[test]
    fn video_defaults_when_fields_missing() {
        let json = br#"{"streams":[{"index":0,"codec_type":"video"}]}"#;
        let v = &parse(json).video[0];
        assert_eq!(v.codec, "");
        assert_eq!(v.width, 0);
        assert_eq!(v.height, 0);
        assert_eq!(v.bit_depth, 8);
        assert_eq!(v.hdr, None);
        assert_eq!(v.bitrate, None);
        assert!((v.frame_rate - 0.0).abs() < f32::EPSILON);
    }

    fn depth_of(stream: &str, table: &str) -> u8 {
        let json = format!(
            r#"{{"streams":[{{"index":0,"codec_type":"video",{stream}}}],"pixel_formats":[{table}]}}"#
        );
        parse(json.as_bytes()).video[0].bit_depth
    }

    const TABLE: &str = r#"
        {"name":"yuv420p","components":[{"index":1,"bit_depth":8},{"index":2,"bit_depth":8}]},
        {"name":"yuv420p10le","components":[{"index":1,"bit_depth":10},{"index":2,"bit_depth":10}]},
        {"name":"yuv420p12le","components":[{"index":1,"bit_depth":12},{"index":2,"bit_depth":12}]},
        {"name":"vaapi","nb_components":0}"#;

    #[test]
    fn the_depth_comes_from_the_pixel_format_when_ffprobe_gives_no_raw_depth() {
        assert_eq!(depth_of(r#""codec_name":"hevc","pix_fmt":"yuv420p10le""#, TABLE), 10);
        assert_eq!(depth_of(r#""codec_name":"hevc","pix_fmt":"yuv420p12le""#, TABLE), 12);
        assert_eq!(depth_of(r#""codec_name":"vp9","pix_fmt":"yuv420p""#, TABLE), 8);
    }

    #[test]
    fn the_raw_depth_is_taken_when_ffprobe_gives_one() {
        assert_eq!(depth_of(r#""codec_name":"h264","bits_per_raw_sample":"10""#, ""), 10);
        assert_eq!(
            depth_of(
                r#""codec_name":"h264","bits_per_raw_sample":"10","pix_fmt":"yuv420p10le""#,
                TABLE
            ),
            10
        );
    }

    #[test]
    fn a_depth_nothing_names_is_eight() {
        assert_eq!(depth_of(r#""codec_name":"hevc","pix_fmt":"yuv420p10le""#, ""), 8);
        assert_eq!(depth_of(r#""codec_name":"hevc","pix_fmt":"unheard_of""#, TABLE), 8);
        assert_eq!(depth_of(r#""codec_name":"hevc","pix_fmt":"vaapi""#, TABLE), 8);
        assert_eq!(depth_of(r#""codec_name":"hevc","bits_per_raw_sample":"N/A""#, TABLE), 8);
    }

    #[test]
    fn a_table_entry_without_a_name_or_depth_is_skipped_rather_than_failing_the_probe() {
        let table = r#"{"components":[{"bit_depth":10}]},{"name":"yuv420p10le","components":[{"index":1}]}"#;
        assert_eq!(depth_of(r#""codec_name":"hevc","pix_fmt":"yuv420p10le""#, table), 8);
    }

    #[test]
    fn frame_rate_parsing() {
        let plain = br#"{"streams":[{"index":0,"codec_type":"video","r_frame_rate":"24"}]}"#;
        assert!((parse(plain).video[0].frame_rate - 24.0).abs() < f32::EPSILON);

        let zero = br#"{"streams":[{"index":0,"codec_type":"video","r_frame_rate":"0/0"}]}"#;
        assert!((parse(zero).video[0].frame_rate - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn missing_duration_is_zero() {
        let json = br#"{"streams":[]}"#;
        assert_eq!(parse(json).duration_ms, 0);
    }

    #[test]
    fn malformed_json_is_parse_error() {
        let err = parse_probe(b"not json").unwrap_err();
        assert!(matches!(err, ProbeError::Parse(_)));
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
            let _ = parse_probe(&bytes);
        }

        #[test]
        fn never_panics_on_arbitrary_json_text(text in "\\PC*") {
            let _ = parse_probe(text.as_bytes());
        }
    }
}

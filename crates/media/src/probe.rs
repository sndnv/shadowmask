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
        Self {
            binary: "ffprobe".to_owned(),
        }
    }
}

impl FfprobeMediaProbe {
    pub fn with_binary(binary: impl Into<String>) -> Self {
        Self {
            binary: binary.into(),
        }
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
            ])
            .arg(path)
            .output()
            .await
            .map_err(|e| ProbeError::Backend(e.to_string()))?;
        if !output.status.success() {
            return Err(ProbeError::Backend(
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
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

    let mut video = Vec::new();
    let mut audio = Vec::new();
    let mut subtitles = Vec::new();
    for stream in parsed.streams {
        match stream.codec_type.as_str() {
            "video" => video.push(video_track(stream)),
            "audio" => audio.push(audio_track(stream)),
            "subtitle" => subtitles.push(subtitle_track(stream)),
            _ => {}
        }
    }

    let chapters = parsed.chapters.into_iter().map(chapter).collect();

    Ok(ProbeResult {
        duration_ms,
        video,
        audio,
        subtitles,
        chapters,
    })
}

fn video_track(s: FfStream) -> VideoTrack {
    VideoTrack {
        index: s.index,
        codec: s.codec_name.unwrap_or_default(),
        width: s.width.unwrap_or(0),
        height: s.height.unwrap_or(0),
        bit_depth: s
            .bits_per_raw_sample
            .as_deref()
            .and_then(|b| b.parse().ok())
            .unwrap_or(8),
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
        language: s.tags.language.map(LanguageCode),
        bitrate: parse_bitrate(s.bit_rate.as_deref()),
    }
}

fn subtitle_track(s: FfStream) -> EmbeddedSubtitleTrack {
    EmbeddedSubtitleTrack {
        index: s.index,
        language: s.tags.language.map(LanguageCode),
        format: subtitle_format(s.codec_name.as_deref()),
        forced: s.disposition.forced == 1,
        default: s.disposition.default == 1,
    }
}

fn chapter(c: FfChapter) -> Chapter {
    Chapter {
        title: c.tags.title.unwrap_or_default(),
        start_ms: seconds_to_ms(c.start_time.as_deref()),
    }
}

fn seconds_to_ms(raw: Option<&str>) -> u64 {
    raw.and_then(|s| s.parse::<f64>().ok())
        .map(|secs| (secs * 1000.0) as u64)
        .unwrap_or(0)
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
        _ => SubtitleFormat::Srt,
    }
}

fn hdr_format(transfer: Option<&str>, side_data: &[FfSideData]) -> Option<HdrFormat> {
    let has = |needle: &str| {
        side_data
            .iter()
            .filter_map(|s| s.side_data_type.as_deref())
            .any(|t| t.contains(needle))
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
        assert_eq!(a[0].language, Some(LanguageCode("eng".to_owned())));
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
        assert_eq!(s[0].language, Some(LanguageCode("eng".to_owned())));
        assert!(s[0].default);
        assert!(!s[0].forced);
        assert_eq!(s[1].format, SubtitleFormat::Pgs);
        assert!(!s[1].default);
        assert!(s[1].forced);
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
        assert_eq!(
            subtitle_format(Some("hdmv_pgs_subtitle")),
            SubtitleFormat::Pgs
        );
        assert_eq!(
            subtitle_format(Some("dvd_subtitle")),
            SubtitleFormat::VobSub
        );
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

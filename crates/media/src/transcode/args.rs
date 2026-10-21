use domain::session::{SegmentContainer, SegmentPlan, TranscodeSpec};

use crate::transcode::VideoEncoder;

pub(crate) const TARGET_MS: u64 = 4_000;
pub(crate) const INIT_SEGMENT: &str = "init.mp4";
pub(crate) const PRODUCER_PLAYLIST: &str = "producer.m3u8";
pub const DEFAULT_READ_RATE: f64 = 10.0;
pub(crate) const OUTPUT_OFFSET_MS: u64 = 1_000;
const NO_NATURAL_KEYFRAMES: &str = "100000";
const NO_REORDERED_FRAMES: &str = "0";
const MPEG_TS_CLOCK: &str = "1/90000";
const SEEK_LEAD_US: i64 = 1_000;
const DECODE_LOOKBACK_US: u64 = 10_000_000;
const BURN_LOOKBACK_US: u64 = 30_000_000;
const AAC_LAYOUTS: [(&str, u8); 7] =
    [("mono", 1), ("stereo", 2), ("3.0", 3), ("4.0", 4), ("5.0", 5), ("5.1", 6), ("7.1", 8)];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RunWindow {
    pub origin_us: u64,
    pub start_us: i64,
    pub segments: usize,
    pub length_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Burn {
    Text(String),
    Sidecar(String),
    Image { path: String, width: u32, height: u32 },
}

pub(crate) fn media_playlist(plan: &SegmentPlan, container: SegmentContainer) -> String {
    let target =
        plan.segments.iter().map(|s| s.duration_ms.div_ceil(1000)).max().unwrap_or(1).max(1);
    let version = match container {
        SegmentContainer::MpegTs => 3,
        SegmentContainer::Fmp4 => 7,
    };
    let mut out = format!(
        "#EXTM3U\n#EXT-X-VERSION:{version}\n#EXT-X-TARGETDURATION:{target}\n\
         #EXT-X-MEDIA-SEQUENCE:0\n#EXT-X-PLAYLIST-TYPE:VOD\n"
    );
    if container == SegmentContainer::Fmp4 {
        out.push_str(&format!("#EXT-X-MAP:URI=\"{INIT_SEGMENT}\"\n"));
    }
    for (index, segment) in plan.segments.iter().enumerate() {
        let seconds = segment.duration_ms as f64 / 1000.0;
        out.push_str(&format!("#EXTINF:{seconds:.3},\n{}\n", segment_file_name(index, container)));
    }
    out.push_str("#EXT-X-ENDLIST\n");
    out
}

pub(crate) fn segment_file_name(index: usize, container: SegmentContainer) -> String {
    format!("seg_{index:05}.{}", container.segment_extension())
}

pub(crate) fn build_run_args(
    spec: &TranscodeSpec,
    encoder: &VideoEncoder,
    window: RunWindow,
    read_rate: f64,
    out_dir: &str,
    burn: Option<&Burn>,
) -> Vec<String> {
    let RunWindow { origin_us, start_us, length_ms, .. } = window;
    let burn = burn.filter(|_| !spec.copy);
    let mut args: Vec<String> = ["-y", "-v", "error"].map(str::to_owned).to_vec();
    if let VideoEncoder::Vaapi { device } = encoder
        && !spec.copy
    {
        args.extend(["-vaapi_device".to_owned(), device.clone()]);
    }
    if spec.copy && read_rate > 0.0 {
        args.extend(["-readrate".to_owned(), read_rate.to_string()]);
    }
    args.push("-copyts".to_owned());
    if origin_us > 0 {
        args.extend(["-seek_timestamp", "1"].map(str::to_owned));
        if !spec.copy || spec.convert_audio {
            args.push("-noaccurate_seek".to_owned());
        }
        let from = if spec.copy { origin_us } else { decode_from(spec, origin_us) };
        let seek = on_file(start_us, from).saturating_sub(SEEK_LEAD_US);
        args.extend(["-ss".to_owned(), seconds(seek)]);
    }
    if let Some(length_ms) = length_ms.filter(|_| spec.copy) {
        args.extend(["-t".to_owned(), seconds(film_us(length_ms.saturating_mul(1_000)))]);
    }
    args.extend(["-i".to_owned(), spec.input_path.clone()]);
    if let Some(Burn::Image { path, .. }) = burn {
        args.extend(["-seek_timestamp", "1", "-noaccurate_seek"].map(str::to_owned));
        let lookback = on_file(start_us, origin_us.saturating_sub(BURN_LOOKBACK_US));
        args.extend(["-ss".to_owned(), seconds(lookback), "-i".to_owned(), path.clone()]);
    }
    if let Some(idx) = spec.audio_track {
        if !matches!(burn, Some(Burn::Image { .. })) {
            args.extend(["-map".to_owned(), "0:v:0".to_owned()]);
        }
        args.extend(["-map".to_owned(), format!("0:{idx}")]);
    }
    args.push("-sn".to_owned());
    if spec.copy {
        if spec.convert_audio {
            args.extend(["-c:v", "copy"].map(str::to_owned));
            push_audio_args(&mut args, spec, trim_bounds(window));
        } else {
            args.extend(["-c", "copy"].map(str::to_owned));
        }
        if origin_us > 0 {
            args.extend(["-copypriorss", "0"].map(str::to_owned));
        }
    } else {
        push_encode_args(&mut args, spec, encoder, window, burn);
    }
    let offset = film_us(OUTPUT_OFFSET_MS * 1_000).saturating_sub(start_us);
    args.extend(["-avoid_negative_ts", "disabled", "-output_ts_offset"].map(str::to_owned));
    args.extend([seconds(offset), "-muxdelay".to_owned(), "0".to_owned()]);
    args.extend(["-f", "hls", "-hls_segment_type", spec.container.as_str()].map(str::to_owned));
    if spec.container == SegmentContainer::Fmp4 {
        args.extend(["-hls_fmp4_init_filename", INIT_SEGMENT].map(str::to_owned));
    }
    args.extend(["-hls_list_size".to_owned(), "0".to_owned()]);
    args.extend(["-hls_time".to_owned(), (TARGET_MS / 1000).to_string()]);
    args.extend(["-hls_playlist_type", "vod", "-hls_flags", "temp_file"].map(str::to_owned));
    let extension = spec.container.segment_extension();
    args.extend(["-hls_segment_filename".to_owned(), format!("{out_dir}/seg_%05d.{extension}")]);
    args.push(format!("{out_dir}/{PRODUCER_PLAYLIST}"));
    args
}

fn push_encode_args(
    args: &mut Vec<String>,
    spec: &TranscodeSpec,
    encoder: &VideoEncoder,
    window: RunWindow,
    burn: Option<&Burn>,
) {
    let bounds = trim_bounds(window);
    let trim = bounds.as_ref().map(|bounds| format!("trim={bounds}"));
    match burn {
        Some(Burn::Image { width, height, .. }) => {
            let graph = overlay_graph(spec, trim, *width, *height, encoder);
            args.extend(["-filter_complex".to_owned(), graph]);
        }
        _ => {
            if let Some(filter) = video_filter(spec, trim, burn, encoder, window.start_us) {
                args.extend(["-vf".to_owned(), filter]);
            }
        }
    }
    match encoder {
        VideoEncoder::Vaapi { .. } => args.extend(["-c:v", "h264_vaapi"].map(str::to_owned)),
        VideoEncoder::Software => args.extend(
            ["-c:v", "libx264", "-preset", "veryfast", "-pix_fmt", "yuv420p", "-sc_threshold", "0"]
                .map(str::to_owned),
        ),
    }
    args.extend(["-g", NO_NATURAL_KEYFRAMES, "-bf", NO_REORDERED_FRAMES].map(str::to_owned));
    push_audio_args(args, spec, bounds);
    if let Some(bitrate) = spec.max_bitrate {
        args.extend(["-maxrate".to_owned(), bitrate.to_string()]);
        args.extend(["-bufsize".to_owned(), (bitrate * 2).to_string()]);
    }
    args.extend([
        "-force_key_frames".to_owned(),
        format!("expr:gte(t,n_forced*{})*lt(n_forced,{})", TARGET_MS / 1000, window.segments),
        "-fps_mode".to_owned(),
        "passthrough".to_owned(),
        "-enc_time_base:v".to_owned(),
        MPEG_TS_CLOCK.to_owned(),
    ]);
}

fn push_audio_args(args: &mut Vec<String>, spec: &TranscodeSpec, bounds: Option<String>) {
    let trim = bounds.map(|bounds| format!("atrim={bounds}"));
    let filters: Vec<String> = trim.into_iter().chain([aac_layouts(spec.audio_channels)]).collect();
    args.extend(["-af".to_owned(), filters.join(",")]);
    args.extend(["-c:a", "aac"].map(str::to_owned));
}

fn aac_layouts(channels: Option<u8>) -> String {
    let limit = channels.unwrap_or(u8::MAX).max(1);
    let layouts: Vec<&str> = AAC_LAYOUTS
        .iter()
        .filter(|(_, count)| *count <= limit)
        .map(|(layout, _)| *layout)
        .collect();
    format!("aformat=channel_layouts={}", layouts.join("|"))
}

fn decode_from(spec: &TranscodeSpec, origin_us: u64) -> u64 {
    spec.keyframes_us
        .iter()
        .flatten()
        .copied()
        .filter(|&keyframe| keyframe <= origin_us)
        .max()
        .unwrap_or_else(|| origin_us.saturating_sub(DECODE_LOOKBACK_US))
}

fn trim_bounds(window: RunWindow) -> Option<String> {
    let RunWindow { origin_us, start_us, length_ms, .. } = window;
    let start = (origin_us > 0).then(|| format!("start={}", seconds(on_file(start_us, origin_us))));
    let end = length_ms.map(|length_ms| {
        let end_us = origin_us.saturating_add(length_ms.saturating_mul(1_000));
        format!("end={}", seconds(on_file(start_us, end_us)))
    });
    let bounds: Vec<String> = start.into_iter().chain(end).collect();
    (!bounds.is_empty()).then(|| bounds.join(":"))
}

fn film_us(us: u64) -> i64 {
    i64::try_from(us).unwrap_or(i64::MAX)
}

fn on_file(start_us: i64, film: u64) -> i64 {
    start_us.saturating_add(film_us(film))
}

fn seconds(us: i64) -> String {
    let sign = if us < 0 { "-" } else { "" };
    let magnitude = us.unsigned_abs();
    format!("{sign}{}.{:06}", magnitude / 1_000_000, magnitude % 1_000_000)
}

pub(crate) fn build_subtitle_extract_args(input: &str, out_vtt: &str) -> Vec<String> {
    ["-nostdin", "-y", "-i", input, "-f", "webvtt", out_vtt].map(str::to_owned).to_vec()
}

pub(crate) fn subtitle_media_playlist(duration_ms: u64, vtt_name: &str) -> String {
    let seconds = duration_ms as f64 / 1000.0;
    let target = duration_ms.div_ceil(1000).max(1);
    format!(
        "#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:{target}\n\
         #EXT-X-MEDIA-SEQUENCE:0\n#EXT-X-PLAYLIST-TYPE:VOD\n\
         #EXTINF:{seconds:.3},\n{vtt_name}\n#EXT-X-ENDLIST\n"
    )
}

const TONEMAP_TO_SDR: &str = "zscale=t=linear:npl=100,tonemap=tonemap=hable:desat=0,\
     zscale=t=bt709:m=bt709:p=bt709:r=tv,format=yuv420p";

fn base_filters(spec: &TranscodeSpec) -> Vec<String> {
    let mut filters = Vec::new();
    if let Some(height) = spec.max_height {
        filters.push(format!("scale=-2:{height}"));
    }
    if spec.source_hdr.is_some() {
        filters.push(TONEMAP_TO_SDR.to_owned());
    }
    filters
}

fn upload_filters(encoder: &VideoEncoder) -> Vec<String> {
    match encoder.is_hardware() {
        true => vec!["format=nv12".to_owned(), "hwupload".to_owned()],
        false => Vec::new(),
    }
}

fn video_filter(
    spec: &TranscodeSpec,
    trim: Option<String>,
    burn: Option<&Burn>,
    encoder: &VideoEncoder,
    start_us: i64,
) -> Option<String> {
    let mut filters: Vec<String> = trim.into_iter().collect();
    filters.extend(base_filters(spec));
    match burn {
        Some(Burn::Text(path)) => filters.push(subtitles(path)),
        Some(Burn::Sidecar(path)) if start_us != 0 => {
            let start = seconds(start_us);
            filters.push(format!("setpts=PTS-({start})/TB"));
            filters.push(subtitles(path));
            filters.push(format!("setpts=PTS+({start})/TB"));
        }
        Some(Burn::Sidecar(path)) => filters.push(subtitles(path)),
        _ => {}
    }
    filters.extend(upload_filters(encoder));
    if filters.is_empty() { None } else { Some(filters.join(",")) }
}

fn subtitles(path: &str) -> String {
    format!("subtitles=filename='{}'", escape_subtitle_path(path))
}

fn overlay_graph(
    spec: &TranscodeSpec,
    trim: Option<String>,
    width: u32,
    height: u32,
    encoder: &VideoEncoder,
) -> String {
    let mut base: Vec<String> = trim.into_iter().collect();
    base.extend(base_filters(spec));
    let base = if base.is_empty() { "null".to_owned() } else { base.join(",") };
    let mut sub = vec![format!("scale={width}:{height}")];
    if let Some(height) = spec.max_height {
        sub.push(format!("scale=-2:{height}"));
    }
    let mut overlay = vec!["overlay=eof_action=pass".to_owned()];
    overlay.extend(upload_filters(encoder));
    format!("[0:v:0]{base}[base];[1:s:0]{}[sub];[base][sub]{}", sub.join(","), overlay.join(","))
}

fn escape_subtitle_path(path: &str) -> String {
    path.replace('\\', "\\\\").replace(':', "\\:").replace('\'', "'\\''")
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::VersionId;
    use domain::media::HdrFormat;
    use domain::session::{SegmentContainer, SessionId, StreamGeneration, plan_cuts};

    const OUT: &str = "/cache/s1/1/v0";
    const EVERY_AAC_LAYOUT: &str = "aformat=channel_layouts=mono|stereo|3.0|4.0|5.0|5.1|7.1";

    fn trimmed(bounds: &str) -> String {
        format!("atrim={bounds},{EVERY_AAC_LAYOUT}")
    }

    fn base_spec() -> TranscodeSpec {
        TranscodeSpec {
            session: SessionId("s1".to_owned()),
            generation: StreamGeneration(1),
            version: VersionId("ver-1".to_owned()),
            input_path: "/media/movie.mkv".to_owned(),
            duration_ms: 120_000,
            keyframes_us: None,
            copy: false,
            container: SegmentContainer::MpegTs,
            seek_ms: None,
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

    fn copy_spec(container: SegmentContainer) -> TranscodeSpec {
        TranscodeSpec { copy: true, container, ..base_spec() }
    }

    fn vaapi() -> VideoEncoder {
        VideoEncoder::Vaapi { device: "/dev/dri/renderD128".to_owned() }
    }

    fn from(origin_ms: u64) -> RunWindow {
        RunWindow { origin_us: origin_ms * 1_000, start_us: 0, segments: 30, length_ms: None }
    }

    fn on_clock(origin_us: u64, start_us: i64) -> RunWindow {
        RunWindow { origin_us, start_us, segments: 30, length_ms: None }
    }

    fn run(spec: &TranscodeSpec, window: RunWindow, burn: Option<&Burn>) -> Vec<String> {
        build_run_args(spec, &VideoEncoder::Software, window, 10.0, OUT, burn)
    }

    fn args_at(spec: &TranscodeSpec, origin_ms: u64, read_rate: f64) -> Vec<String> {
        build_run_args(spec, &VideoEncoder::Software, from(origin_ms), read_rate, OUT, None)
    }

    fn args_for(spec: &TranscodeSpec) -> Vec<String> {
        args_at(spec, 0, DEFAULT_READ_RATE)
    }

    fn vaapi_args_for(spec: &TranscodeSpec) -> Vec<String> {
        build_run_args(spec, &vaapi(), from(0), DEFAULT_READ_RATE, OUT, None)
    }

    fn burned(
        spec: &TranscodeSpec,
        burn: &Burn,
        encoder: &VideoEncoder,
        origin_ms: u64,
    ) -> Vec<String> {
        build_run_args(spec, encoder, from(origin_ms), DEFAULT_READ_RATE, OUT, Some(burn))
    }

    fn text(path: &str) -> Burn {
        Burn::Text(path.to_owned())
    }

    fn image() -> Burn {
        Burn::Image { path: "/prep/ver-1/embedded-3.mks".to_owned(), width: 1920, height: 800 }
    }

    fn sidecar(path: &str) -> Burn {
        Burn::Sidecar(path.to_owned())
    }

    fn text_vf(spec: &TranscodeSpec, encoder: &VideoEncoder) -> String {
        pair_after(&burned(spec, &sidecar("/media/movie.srt"), encoder, 0), "-vf").expect("-vf")
    }

    fn graph(spec: &TranscodeSpec, encoder: &VideoEncoder) -> String {
        pair_after(&burned(spec, &image(), encoder, 0), "-filter_complex").expect("a graph")
    }

    fn inputs(args: &[String]) -> Vec<usize> {
        args.iter().enumerate().filter(|(_, a)| *a == "-i").map(|(i, _)| i).collect()
    }

    fn maps(args: &[String]) -> Vec<&str> {
        args.iter()
            .enumerate()
            .filter(|(_, a)| *a == "-map")
            .map(|(i, _)| args[i + 1].as_str())
            .collect()
    }

    fn pair_after(args: &[String], flag: &str) -> Option<String> {
        args.iter().position(|a| a == flag).map(|i| args[i + 1].clone())
    }

    fn at(args: &[String], flag: &str) -> usize {
        args.iter().position(|a| a == flag).unwrap_or_else(|| panic!("{flag} is missing"))
    }

    #[test]
    fn every_run_leaves_the_film_subtitles_out() {
        for spec in [base_spec(), copy_spec(SegmentContainer::MpegTs)] {
            let args = args_for(&spec);
            assert!(
                at(&args, "-sn") > at(&args, "-i"),
                "without it ffmpeg picks a text track when no audio track is mapped, and a remux fails"
            );
        }
    }

    #[test]
    fn a_transcode_run_writes_mpegts_segments_through_the_hls_muxer() {
        let args = args_for(&base_spec());
        assert_eq!(args[..3], ["-y", "-v", "error"]);
        assert_eq!(pair_after(&args, "-i").as_deref(), Some("/media/movie.mkv"));
        assert_eq!(pair_after(&args, "-c:v").as_deref(), Some("libx264"));
        assert_eq!(pair_after(&args, "-preset").as_deref(), Some("veryfast"));
        assert_eq!(pair_after(&args, "-pix_fmt").as_deref(), Some("yuv420p"));
        assert_eq!(pair_after(&args, "-c:a").as_deref(), Some("aac"));
        assert_eq!(pair_after(&args, "-f").as_deref(), Some("hls"));
        assert_eq!(pair_after(&args, "-hls_segment_type").as_deref(), Some("mpegts"));
        assert_eq!(pair_after(&args, "-hls_time").as_deref(), Some("4"));
        assert_eq!(pair_after(&args, "-hls_list_size").as_deref(), Some("0"));
        assert_eq!(pair_after(&args, "-hls_playlist_type").as_deref(), Some("vod"));
        assert_eq!(pair_after(&args, "-hls_flags").as_deref(), Some("temp_file"));
        assert_eq!(
            pair_after(&args, "-hls_segment_filename").as_deref(),
            Some("/cache/s1/1/v0/seg_%05d.ts")
        );
        assert_eq!(args.last().map(String::as_str), Some("/cache/s1/1/v0/producer.m3u8"));
        assert!(!args.iter().any(|a| a == "-hls_fmp4_init_filename"));
        assert!(!args.iter().any(|a| a == "-map" || a == "-vf" || a == "-maxrate"));
        assert!(!args.iter().any(|a| a == "-ss" || a == "-t" || a == "-to"));
    }

    #[test]
    fn every_run_reads_the_file_clock_and_moves_the_film_start_to_one_second() {
        for spec in [base_spec(), copy_spec(SegmentContainer::MpegTs)] {
            for (start_us, offset) in
                [(0, "1.000000"), (1_400_000, "-0.400000"), (-23_220, "1.023220")]
            {
                let args = run(&spec, on_clock(0, start_us), None);
                let input = at(&args, "-i");
                assert!(at(&args, "-copyts") < input);
                assert!(!args.iter().any(|a| a == "-start_at_zero" || a == "-itsoffset"));
                assert_eq!(pair_after(&args, "-avoid_negative_ts").as_deref(), Some("disabled"));
                assert_eq!(pair_after(&args, "-output_ts_offset").as_deref(), Some(offset));
                assert_eq!(pair_after(&args, "-muxdelay").as_deref(), Some("0"));
                assert!(at(&args, "-output_ts_offset") > input);
            }
        }
    }

    #[test]
    fn a_transcode_forces_a_keyframe_on_each_planned_boundary_and_nowhere_else() {
        let window = RunWindow { origin_us: 0, start_us: 0, segments: 7, length_ms: None };
        let args = build_run_args(&base_spec(), &VideoEncoder::Software, window, 10.0, OUT, None);
        assert_eq!(
            pair_after(&args, "-force_key_frames").as_deref(),
            Some("expr:gte(t,n_forced*4)*lt(n_forced,7)")
        );
        assert_eq!(pair_after(&args, "-sc_threshold").as_deref(), Some("0"));
        assert_eq!(pair_after(&args, "-g").as_deref(), Some(NO_NATURAL_KEYFRAMES));
        assert_eq!(pair_after(&args, "-fps_mode").as_deref(), Some("passthrough"));
        assert!(!args.iter().any(|a| a == "-readrate"));
    }

    #[test]
    fn every_encode_leaves_out_reordered_frames_so_each_segment_opens_on_its_keyframe() {
        let window = RunWindow { origin_us: 0, start_us: 0, segments: 7, length_ms: None };
        for encoder in [VideoEncoder::Software, vaapi()] {
            let args = build_run_args(&base_spec(), &encoder, window, 10.0, OUT, None);
            assert_eq!(pair_after(&args, "-bf").as_deref(), Some(NO_REORDERED_FRAMES));
        }
        for burn in [sidecar("/media/movie.srt"), image()] {
            let args = burned(&base_spec(), &burn, &vaapi(), 0);
            assert_eq!(pair_after(&args, "-bf").as_deref(), Some(NO_REORDERED_FRAMES));
        }
        for container in [SegmentContainer::MpegTs, SegmentContainer::Fmp4] {
            let copy = vaapi_args_for(&copy_spec(container));
            assert!(!copy.iter().any(|a| a == "-bf"));
            let converted =
                vaapi_args_for(&TranscodeSpec { convert_audio: true, ..copy_spec(container) });
            assert!(!converted.iter().any(|a| a == "-bf"));
        }
    }

    #[test]
    fn every_encode_keeps_the_mpegts_clock_so_no_two_frames_share_a_time() {
        let window = RunWindow { origin_us: 0, start_us: 0, segments: 7, length_ms: None };
        for encoder in [VideoEncoder::Software, vaapi()] {
            let args = build_run_args(&base_spec(), &encoder, window, 10.0, OUT, None);
            assert_eq!(pair_after(&args, "-enc_time_base:v").as_deref(), Some("1/90000"));
        }
        for burn in [sidecar("/media/movie.srt"), image()] {
            let args = burned(&base_spec(), &burn, &vaapi(), 9_000);
            assert_eq!(pair_after(&args, "-enc_time_base:v").as_deref(), Some("1/90000"));
        }
        for container in [SegmentContainer::MpegTs, SegmentContainer::Fmp4] {
            let copy = args_for(&copy_spec(container));
            assert!(!copy.iter().any(|a| a.starts_with("-enc_time_base")));
            let converted =
                args_for(&TranscodeSpec { convert_audio: true, ..copy_spec(container) });
            assert!(!converted.iter().any(|a| a.starts_with("-enc_time_base")));
        }
    }

    #[test]
    fn a_transcode_decodes_from_the_keyframe_before_its_origin_and_trims_to_it() {
        let spec = TranscodeSpec {
            keyframes_us: Some(vec![343_222, 4_023_222, 8_023_222]),
            audio_track: Some(1),
            ..base_spec()
        };
        let args = run(&spec, on_clock(8_000_000, 1_400_000), None);
        let input = at(&args, "-i");
        assert_eq!(
            args[at(&args, "-copyts")..input],
            ["-copyts", "-seek_timestamp", "1", "-noaccurate_seek", "-ss", "5.422222"]
        );
        assert_eq!(pair_after(&args, "-vf").as_deref(), Some("trim=start=9.400000"));
        assert_eq!(pair_after(&args, "-af"), Some(trimmed("start=9.400000")));
        assert!(at(&args, "-af") > input);
        assert!(!args.iter().any(|a| a == "-copypriorss" || a == "-t"));
    }

    #[test]
    fn a_transcode_without_keyframes_decodes_from_ten_seconds_back() {
        let args = run(&base_spec(), on_clock(60_000_000, 0), None);
        assert_eq!(pair_after(&args, "-ss").as_deref(), Some("49.999000"));
        assert_eq!(pair_after(&args, "-vf").as_deref(), Some("trim=start=60.000000"));
        let early = run(&base_spec(), on_clock(4_000_000, 0), None);
        assert_eq!(pair_after(&early, "-ss").as_deref(), Some("-0.001000"));
    }

    #[test]
    fn a_transcode_from_the_start_neither_seeks_nor_trims() {
        let args = run(&base_spec(), on_clock(0, 1_400_000), None);
        assert!(!args.iter().any(|a| a == "-ss" || a == "-vf"));
        assert_eq!(pair_after(&args, "-af").as_deref(), Some(EVERY_AAC_LAYOUT));
    }

    #[test]
    fn a_copy_starts_a_millisecond_before_its_keyframe_on_the_file_clock_and_drops_what_precedes_it()
     {
        let spec = copy_spec(SegmentContainer::MpegTs);
        let args = run(&spec, on_clock(8_333_417, 9_976_780), None);
        let input = at(&args, "-i");
        assert_eq!(
            args[at(&args, "-copyts")..input],
            ["-copyts", "-seek_timestamp", "1", "-ss", "18.309197"]
        );
        assert_eq!(pair_after(&args, "-copypriorss").as_deref(), Some("0"));
        assert!(at(&args, "-copypriorss") > input);
        assert!(!args.iter().any(|a| a == "-noaccurate_seek" || a == "-af"));
    }

    #[test]
    fn a_copy_from_the_start_neither_seeks_nor_drops() {
        let args = args_for(&copy_spec(SegmentContainer::MpegTs));
        assert!(!args.iter().any(|a| a == "-ss" || a == "-copypriorss"));
    }

    #[test]
    fn a_copy_streams_without_encoding_or_filters() {
        let spec = TranscodeSpec {
            max_height: Some(720),
            max_bitrate: Some(4_000_000),
            audio_channels: Some(2),
            source_hdr: Some(HdrFormat::Hdr10Plus),
            ..copy_spec(SegmentContainer::MpegTs)
        };
        let args = vaapi_args_for(&spec);
        assert_eq!(pair_after(&args, "-c").as_deref(), Some("copy"));
        for flag in ["-c:v", "-c:a", "-vf", "-af", "-maxrate", "-force_key_frames", "-g"] {
            assert!(!args.iter().any(|a| a == flag), "{flag}");
        }
        assert!(!args.iter().any(|a| a == "-vaapi_device"));
    }

    #[test]
    fn a_copy_converting_its_sound_copies_the_picture_and_encodes_aac() {
        let spec = TranscodeSpec {
            convert_audio: true,
            audio_channels: Some(6),
            audio_track: Some(2),
            max_height: Some(720),
            ..copy_spec(SegmentContainer::MpegTs)
        };
        let args = vaapi_args_for(&spec);
        assert_eq!(maps(&args), ["0:v:0", "0:2"]);
        assert_eq!(pair_after(&args, "-c:v").as_deref(), Some("copy"));
        assert_eq!(pair_after(&args, "-c:a").as_deref(), Some("aac"));
        assert_eq!(
            pair_after(&args, "-af").as_deref(),
            Some("aformat=channel_layouts=mono|stereo|3.0|4.0|5.0|5.1")
        );
        assert_eq!(pair_after(&args, "-readrate").as_deref(), Some("10"));
        for flag in ["-c", "-ac", "-ss", "-noaccurate_seek", "-copypriorss", "-vaapi_device"] {
            assert!(!args.iter().any(|a| a == flag), "{flag}");
        }
        for flag in ["-vf", "-maxrate", "-force_key_frames", "-g"] {
            assert!(!args.iter().any(|a| a == flag), "{flag}");
        }
    }

    #[test]
    fn a_resumed_copy_converting_its_sound_trims_it_to_the_keyframe_on_the_file_clock() {
        let spec = TranscodeSpec { convert_audio: true, ..copy_spec(SegmentContainer::Fmp4) };
        let args = run(&spec, on_clock(8_333_417, 9_976_780), None);
        let input = at(&args, "-i");
        assert_eq!(
            args[at(&args, "-copyts")..input],
            ["-copyts", "-seek_timestamp", "1", "-noaccurate_seek", "-ss", "18.309197"]
        );
        assert_eq!(pair_after(&args, "-af"), Some(trimmed("start=18.310197")));
        assert_eq!(pair_after(&args, "-copypriorss").as_deref(), Some("0"));
        assert!(at(&args, "-af") > input);
        assert!(!args.iter().any(|a| a == "-ac" || a == "-vf"));
    }

    #[test]
    fn a_copy_is_paced_by_the_read_rate() {
        let args = args_for(&copy_spec(SegmentContainer::MpegTs));
        assert_eq!(pair_after(&args, "-readrate").as_deref(), Some("10"));
        assert!(at(&args, "-readrate") < at(&args, "-i"));
        let spec = copy_spec(SegmentContainer::MpegTs);
        let custom = args_at(&spec, 0, 2.5);
        assert_eq!(pair_after(&custom, "-readrate").as_deref(), Some("2.5"));
    }

    #[test]
    fn a_read_rate_of_zero_lets_a_copy_run_flat_out() {
        let args = args_at(&copy_spec(SegmentContainer::MpegTs), 0, 0.0);
        assert!(!args.iter().any(|a| a == "-readrate"));
    }

    #[test]
    fn an_fmp4_run_names_its_init_and_segments_like_the_playlist() {
        let args = args_for(&copy_spec(SegmentContainer::Fmp4));
        assert_eq!(pair_after(&args, "-hls_segment_type").as_deref(), Some("fmp4"));
        assert_eq!(pair_after(&args, "-hls_fmp4_init_filename").as_deref(), Some("init.mp4"));
        assert_eq!(
            pair_after(&args, "-hls_segment_filename").as_deref(),
            Some("/cache/s1/1/v0/seg_%05d.m4s")
        );
    }

    #[test]
    fn a_transcode_with_a_length_trims_its_end_on_the_file_clock() {
        let window = RunWindow {
            origin_us: 8_000_000,
            start_us: 1_400_000,
            segments: 3,
            length_ms: Some(12_000),
        };
        let args = run(&base_spec(), window, None);
        assert_eq!(pair_after(&args, "-vf").as_deref(), Some("trim=start=9.400000:end=21.400000"));
        assert_eq!(pair_after(&args, "-af"), Some(trimmed("start=9.400000:end=21.400000")));
        assert!(!args.iter().any(|a| a == "-t"));
        let first = RunWindow { origin_us: 0, ..window };
        let args = run(&base_spec(), first, None);
        assert_eq!(pair_after(&args, "-vf").as_deref(), Some("trim=end=13.400000"));
    }

    #[test]
    fn a_copy_with_a_length_reads_only_that_much() {
        let window =
            RunWindow { origin_us: 8_000_000, start_us: 0, segments: 3, length_ms: Some(12_000) };
        let args = run(&copy_spec(SegmentContainer::MpegTs), window, None);
        assert_eq!(pair_after(&args, "-t").as_deref(), Some("12.000000"));
        assert!(at(&args, "-t") < at(&args, "-i"));
    }

    #[test]
    fn the_requested_audio_track_is_mapped_with_the_first_video() {
        for spec in [base_spec(), copy_spec(SegmentContainer::Fmp4)] {
            let args = args_for(&TranscodeSpec { audio_track: Some(2), ..spec });
            assert_eq!(maps(&args), vec!["0:v:0", "0:2"]);
        }
    }

    #[test]
    fn max_height_adds_a_scale_filter() {
        let spec = TranscodeSpec { max_height: Some(720), ..base_spec() };
        assert_eq!(pair_after(&args_for(&spec), "-vf").as_deref(), Some("scale=-2:720"));
    }

    #[test]
    fn a_burn_adds_the_subtitles_filter_after_the_scale() {
        let spec = TranscodeSpec { max_height: Some(480), ..base_spec() };
        assert_eq!(
            text_vf(&spec, &VideoEncoder::Software),
            "scale=-2:480,subtitles=filename='/media/movie.srt'"
        );
    }

    #[test]
    fn a_copied_text_track_is_drawn_on_the_file_clock_it_was_copied_with() {
        let spec = TranscodeSpec { audio_track: Some(1), ..base_spec() };
        let burn = text("/prep/ver-1/embedded-3.mks");
        let args = run(&spec, on_clock(8_000_000, 1_400_000), Some(&burn));
        assert_eq!(
            pair_after(&args, "-vf").as_deref(),
            Some("trim=start=9.400000,subtitles=filename='/prep/ver-1/embedded-3.mks'")
        );
    }

    #[test]
    fn a_sidecar_is_drawn_on_the_film_clock_of_a_file_that_starts_late() {
        let burn = sidecar("/media/movie.srt");
        let args = run(&base_spec(), on_clock(0, 1_400_000), Some(&burn));
        assert_eq!(
            pair_after(&args, "-vf").as_deref(),
            Some(
                "setpts=PTS-(1.400000)/TB,subtitles=filename='/media/movie.srt',setpts=PTS+(1.400000)/TB"
            )
        );
        let early = run(&base_spec(), on_clock(0, -23_220), Some(&burn));
        assert!(pair_after(&early, "-vf").unwrap().starts_with("setpts=PTS-(-0.023220)/TB,"));
    }

    #[test]
    fn a_burn_escapes_hostile_paths() {
        let vf = |name: &str| {
            let args = burned(&base_spec(), &sidecar(name), &VideoEncoder::Software, 0);
            pair_after(&args, "-vf").expect("-vf")
        };
        assert_eq!(
            vf("/m/Mission: Impossible.srt"),
            "subtitles=filename='/m/Mission\\: Impossible.srt'"
        );
        assert_eq!(vf("/m/Bob's tape.srt"), "subtitles=filename='/m/Bob'\\''s tape.srt'");
        assert_eq!(vf("/m/a,b.srt"), "subtitles=filename='/m/a,b.srt'");
        assert_eq!(vf("/m/a\\b.srt"), "subtitles=filename='/m/a\\\\b.srt'");
    }

    #[test]
    fn max_bitrate_adds_maxrate_and_bufsize() {
        let spec = TranscodeSpec { max_bitrate: Some(4_000_000), ..base_spec() };
        let args = args_for(&spec);
        assert_eq!(pair_after(&args, "-maxrate").as_deref(), Some("4000000"));
        assert_eq!(pair_after(&args, "-bufsize").as_deref(), Some("8000000"));
    }

    #[test]
    fn vaapi_uses_the_hardware_encoder_and_uploads_last() {
        let spec = TranscodeSpec { max_height: Some(480), ..base_spec() };
        let args = burned(&spec, &sidecar("/media/movie.srt"), &vaapi(), 0);
        assert_eq!(pair_after(&args, "-vaapi_device").as_deref(), Some("/dev/dri/renderD128"));
        assert!(at(&args, "-vaapi_device") < at(&args, "-i"));
        assert_eq!(pair_after(&args, "-c:v").as_deref(), Some("h264_vaapi"));
        assert_eq!(
            pair_after(&args, "-vf").as_deref(),
            Some("scale=-2:480,subtitles=filename='/media/movie.srt',format=nv12,hwupload")
        );
        assert!(!args.iter().any(|a| a == "libx264" || a == "-preset" || a == "-sc_threshold"));
        assert_eq!(
            pair_after(&args, "-force_key_frames").as_deref(),
            Some("expr:gte(t,n_forced*4)*lt(n_forced,30)")
        );
        assert_eq!(pair_after(&args, "-g").as_deref(), Some(NO_NATURAL_KEYFRAMES));
    }

    #[test]
    fn an_hdr_source_is_tonemapped_after_the_scale_and_before_the_burn() {
        let spec = TranscodeSpec {
            source_hdr: Some(HdrFormat::Hlg),
            max_height: Some(480),
            ..base_spec()
        };
        assert_eq!(
            text_vf(&spec, &VideoEncoder::Software),
            format!("scale=-2:480,{TONEMAP_TO_SDR},subtitles=filename='/media/movie.srt'")
        );
        let dolby = TranscodeSpec { source_hdr: Some(HdrFormat::DolbyVision), ..base_spec() };
        assert_eq!(
            pair_after(&vaapi_args_for(&dolby), "-vf").as_deref(),
            Some(format!("{TONEMAP_TO_SDR},format=nv12,hwupload").as_str())
        );
    }

    #[test]
    fn an_image_burn_reads_its_track_from_a_second_input_seeked_back_on_the_file_clock() {
        let image = image();
        let args = run(&base_spec(), on_clock(60_000_000, 1_400_000), Some(&image));
        let inputs = inputs(&args);
        assert_eq!(inputs.len(), 2);
        assert_eq!(args[inputs[0] + 1], "/media/movie.mkv");
        assert_eq!(args[inputs[1] + 1], "/prep/ver-1/embedded-3.mks");
        assert_eq!(
            args[inputs[0] + 2..inputs[1]],
            ["-seek_timestamp", "1", "-noaccurate_seek", "-ss", "31.400000"]
        );
        assert_eq!(pair_after(&args, "-ss").as_deref(), Some("51.399000"));
        assert!(at(&args, "-ss") < inputs[0]);
        assert!(!args.iter().any(|a| a == "-vf" || a == "-itsoffset"));
        assert_eq!(
            pair_after(&args, "-filter_complex").as_deref(),
            Some(
                "[0:v:0]trim=start=61.400000[base];[1:s:0]scale=1920:800[sub];[base][sub]overlay=eof_action=pass"
            )
        );
    }

    #[test]
    fn an_image_lookback_stops_at_the_film_start() {
        let image = image();
        let args = run(&base_spec(), on_clock(8_000_000, 1_400_000), Some(&image));
        let inputs = inputs(&args);
        assert_eq!(
            args[inputs[0] + 2..inputs[1]],
            ["-seek_timestamp", "1", "-noaccurate_seek", "-ss", "1.400000"]
        );
    }

    #[test]
    fn an_image_burn_stretches_the_subtitle_canvas_to_the_video_and_leaves_the_output_unlabelled() {
        assert_eq!(
            graph(&base_spec(), &VideoEncoder::Software),
            "[0:v:0]null[base];[1:s:0]scale=1920:800[sub];[base][sub]overlay=eof_action=pass"
        );
    }

    #[test]
    fn an_image_burn_scales_the_subtitle_with_the_video_and_overlays_after_the_tonemap() {
        let spec = TranscodeSpec {
            source_hdr: Some(HdrFormat::Hdr10),
            max_height: Some(480),
            ..base_spec()
        };
        assert_eq!(
            graph(&spec, &VideoEncoder::Software),
            format!(
                "[0:v:0]scale=-2:480,{TONEMAP_TO_SDR}[base];[1:s:0]scale=1920:800,scale=-2:480[sub];[base][sub]overlay=eof_action=pass"
            )
        );
    }

    #[test]
    fn an_image_burn_uploads_for_vaapi_after_the_overlay() {
        assert_eq!(
            graph(&base_spec(), &vaapi()),
            "[0:v:0]null[base];[1:s:0]scale=1920:800[sub];[base][sub]overlay=eof_action=pass,format=nv12,hwupload"
        );
    }

    #[test]
    fn an_image_burn_maps_only_a_chosen_audio_track_beside_the_overlaid_video() {
        let spec = TranscodeSpec { audio_track: Some(2), ..base_spec() };
        assert_eq!(maps(&burned(&spec, &image(), &VideoEncoder::Software, 0)), vec!["0:2"]);
        let args = burned(&base_spec(), &image(), &VideoEncoder::Software, 0);
        assert!(maps(&args).is_empty(), "ffmpeg then picks the audio as it does without a burn");
        assert!(at(&args, "-sn") > inputs(&args)[1]);
        let resumed = burned(&spec, &image(), &VideoEncoder::Software, 8_000);
        assert_eq!(pair_after(&resumed, "-af"), Some(trimmed("start=8.000000")));
    }

    #[test]
    fn a_stream_copy_never_burns() {
        let spec = copy_spec(SegmentContainer::MpegTs);
        for burn in [image(), text("/prep/ver-1/embedded-3.mks"), sidecar("/media/movie.srt")] {
            let args = burned(&spec, &burn, &VideoEncoder::Software, 0);
            assert_eq!(inputs(&args).len(), 1);
            assert!(!args.iter().any(|a| a == "-filter_complex" || a == "-vf"));
        }
    }

    #[test]
    fn a_transcode_limits_its_sound_to_the_channels_asked_for() {
        let spec = TranscodeSpec { audio_channels: Some(2), ..base_spec() };
        let args = args_for(&spec);
        assert_eq!(
            pair_after(&args, "-af").as_deref(),
            Some("aformat=channel_layouts=mono|stereo")
        );
        assert!(!args.iter().any(|a| a == "-ac"));
    }

    #[test]
    fn every_aac_encode_is_held_to_a_layout_aac_has_a_configuration_for() {
        let converting =
            TranscodeSpec { convert_audio: true, ..copy_spec(SegmentContainer::MpegTs) };
        for spec in [base_spec(), converting] {
            let args = args_for(&spec);
            assert_eq!(pair_after(&args, "-af").as_deref(), Some(EVERY_AAC_LAYOUT));
            assert!(!args.iter().any(|a| a == "-ac"));
        }
        for (channels, layouts) in [
            (0, "mono"),
            (1, "mono"),
            (3, "mono|stereo|3.0"),
            (7, "mono|stereo|3.0|4.0|5.0|5.1"),
            (8, "mono|stereo|3.0|4.0|5.0|5.1|7.1"),
        ] {
            let spec = TranscodeSpec { audio_channels: Some(channels), ..base_spec() };
            assert_eq!(
                pair_after(&args_for(&spec), "-af"),
                Some(format!("aformat=channel_layouts={layouts}")),
                "{channels} channels"
            );
        }
    }

    #[test]
    fn a_playlist_is_a_finished_vod_numbered_from_zero() {
        let plan = plan_cuts(&[60_000_000, 64_000_000, 68_000_000], 60_000_000, 70_000, 4_000);
        let playlist = media_playlist(&plan, SegmentContainer::MpegTs);
        assert!(playlist.starts_with("#EXTM3U\n"));
        assert!(playlist.contains("#EXT-X-VERSION:3"));
        assert!(playlist.contains("#EXT-X-PLAYLIST-TYPE:VOD"));
        assert!(playlist.contains("#EXT-X-MEDIA-SEQUENCE:0"));
        assert!(playlist.contains("#EXT-X-TARGETDURATION:4"));
        assert!(playlist.contains("#EXTINF:4.000,\nseg_00000.ts\n"));
        assert!(playlist.contains("#EXTINF:2.000,\nseg_00002.ts\n"));
        assert!(!playlist.contains("seg_00003.ts"));
        assert!(playlist.trim_end().ends_with("#EXT-X-ENDLIST"));
    }

    #[test]
    fn an_empty_playlist_targets_one_second() {
        let playlist =
            media_playlist(&SegmentPlan { segments: Vec::new() }, SegmentContainer::MpegTs);
        assert!(playlist.contains("#EXT-X-TARGETDURATION:1"));
        assert!(playlist.trim_end().ends_with("#EXT-X-ENDLIST"));
    }

    #[test]
    fn an_fmp4_playlist_declares_version_seven_and_maps_the_init_first() {
        let plan = plan_cuts(&[0, 4_000_000], 0, 8_000, 4_000);
        let playlist = media_playlist(&plan, SegmentContainer::Fmp4);
        assert!(playlist.contains("#EXT-X-VERSION:7"));
        let map = playlist.find("#EXT-X-MAP:URI=\"init.mp4\"").expect("the init is mapped");
        let first = playlist.find("seg_00000.m4s").expect("segment zero is listed");
        assert!(map < first);
        assert!(!playlist.contains(".ts"));
    }

    #[test]
    fn segment_file_names_are_zero_padded_and_match_the_container() {
        assert_eq!(segment_file_name(0, SegmentContainer::MpegTs), "seg_00000.ts");
        assert_eq!(segment_file_name(42, SegmentContainer::MpegTs), "seg_00042.ts");
        assert_eq!(segment_file_name(0, SegmentContainer::Fmp4), "seg_00000.m4s");
    }

    #[test]
    fn subtitle_extract_file_reads_sidecar() {
        let args = build_subtitle_extract_args("/media/movie.en.srt", "/cache/s1/subs/subs.vtt");
        assert_eq!(
            args,
            [
                "-nostdin",
                "-y",
                "-i",
                "/media/movie.en.srt",
                "-f",
                "webvtt",
                "/cache/s1/subs/subs.vtt"
            ]
        );
    }

    #[test]
    fn subtitle_playlist_is_single_segment_vod() {
        let playlist = subtitle_media_playlist(125_500, "subs.vtt");
        assert!(playlist.starts_with("#EXTM3U\n"));
        assert!(playlist.contains("#EXT-X-PLAYLIST-TYPE:VOD"));
        assert!(playlist.contains("#EXT-X-TARGETDURATION:126"));
        assert!(playlist.contains("#EXTINF:125.500,\nsubs.vtt\n"));
        assert!(playlist.trim_end().ends_with("#EXT-X-ENDLIST"));
    }
}

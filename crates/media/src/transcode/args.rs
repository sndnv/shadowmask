use domain::session::{Segment, SegmentContainer, SegmentPlan, SoftSubtitleSource, TranscodeSpec};

use crate::transcode::VideoEncoder;

pub(crate) const TARGET_MS: u64 = 4_000;
pub(crate) const INIT_SEGMENT: &str = "init.mp4";
pub(crate) const PRODUCER_PLAYLIST: &str = "producer.m3u8";
pub const DEFAULT_READ_RATE: f64 = 10.0;

pub(crate) fn media_playlist(
    plan: &SegmentPlan,
    container: SegmentContainer,
    first_index: usize,
) -> String {
    let target = plan
        .segments
        .iter()
        .map(|s| s.duration_ms.div_ceil(1000))
        .max()
        .unwrap_or(1)
        .max(1);
    let version = match container {
        SegmentContainer::MpegTs => 3,
        SegmentContainer::Fmp4 => 7,
    };
    let mut out = format!(
        "#EXTM3U\n#EXT-X-VERSION:{version}\n#EXT-X-TARGETDURATION:{target}\n\
         #EXT-X-MEDIA-SEQUENCE:{first_index}\n#EXT-X-PLAYLIST-TYPE:VOD\n"
    );
    if container == SegmentContainer::Fmp4 {
        out.push_str(&format!("#EXT-X-MAP:URI=\"{INIT_SEGMENT}\"\n"));
    }
    for (offset, segment) in plan.segments.iter().enumerate() {
        let seconds = segment.duration_ms as f64 / 1000.0;
        out.push_str(&format!(
            "#EXTINF:{seconds:.3},\n{}\n",
            segment_file_name(first_index + offset, container)
        ));
    }
    out.push_str("#EXT-X-ENDLIST\n");
    out
}

pub(crate) fn segment_file_name(index: usize, container: SegmentContainer) -> String {
    format!("seg_{index:05}.{}", container.segment_extension())
}

pub(crate) fn build_producer_args(
    spec: &TranscodeSpec,
    out_dir: &str,
    origin_ms: u64,
    first_index: usize,
    read_rate: f64,
) -> Vec<String> {
    let mut args = vec!["-y".to_owned(), "-v".to_owned(), "error".to_owned()];
    if read_rate > 0.0 {
        args.push("-readrate".to_owned());
        args.push(read_rate.to_string());
    }
    if origin_ms > 0 {
        args.push("-ss".to_owned());
        args.push(format!("{:.3}", origin_ms as f64 / 1000.0));
    }
    args.push("-i".to_owned());
    args.push(spec.input_path.clone());
    if let Some(idx) = spec.audio_track {
        args.push("-map".to_owned());
        args.push("0:v:0".to_owned());
        args.push("-map".to_owned());
        args.push(format!("0:{idx}"));
    }
    args.push("-c".to_owned());
    args.push("copy".to_owned());
    args.push("-f".to_owned());
    args.push("hls".to_owned());
    args.push("-hls_segment_type".to_owned());
    args.push("fmp4".to_owned());
    args.push("-hls_fmp4_init_filename".to_owned());
    args.push(INIT_SEGMENT.to_owned());
    args.push("-hls_list_size".to_owned());
    args.push("0".to_owned());
    args.push("-hls_time".to_owned());
    args.push((TARGET_MS / 1000).to_string());
    args.push("-hls_playlist_type".to_owned());
    args.push("vod".to_owned());
    args.push("-start_number".to_owned());
    args.push(first_index.to_string());
    args.push("-hls_flags".to_owned());
    args.push("temp_file".to_owned());
    args.push("-hls_segment_filename".to_owned());
    args.push(format!("{out_dir}/seg_%05d.m4s"));
    args.push(format!("{out_dir}/{PRODUCER_PLAYLIST}"));
    args
}

pub(crate) fn build_segment_args(
    spec: &TranscodeSpec,
    segment: &Segment,
    out_path: &str,
    encoder: &VideoEncoder,
) -> Vec<String> {
    let start = segment.start_ms as f64 / 1000.0;
    let last_ms = segment.start_ms + segment.duration_ms;
    let end = if spec.copy {
        last_ms
    } else {
        last_ms.saturating_sub(1)
    } as f64
        / 1000.0;
    let mut args = vec!["-y".to_owned()];
    if let VideoEncoder::Vaapi { device } = encoder
        && !spec.copy
    {
        args.push("-vaapi_device".to_owned());
        args.push(device.clone());
    }
    if spec.copy {
        args.push("-noaccurate_seek".to_owned());
    }
    args.push("-copyts".to_owned());
    args.push("-ss".to_owned());
    args.push(format!("{start:.3}"));
    args.push("-i".to_owned());
    args.push(spec.input_path.clone());
    args.push("-to".to_owned());
    args.push(format!("{end:.3}"));
    if let Some(idx) = spec.audio_track {
        args.push("-map".to_owned());
        args.push("0:v:0".to_owned());
        args.push("-map".to_owned());
        args.push(format!("0:{idx}"));
    }
    if spec.copy {
        args.push("-c".to_owned());
        args.push("copy".to_owned());
    } else {
        if let Some(filter) = video_filter(spec, encoder) {
            args.push("-vf".to_owned());
            args.push(filter);
        }
        match encoder {
            VideoEncoder::Vaapi { .. } => {
                args.push("-c:v".to_owned());
                args.push("h264_vaapi".to_owned());
            }
            VideoEncoder::Software => {
                args.push("-c:v".to_owned());
                args.push("libx264".to_owned());
                args.push("-preset".to_owned());
                args.push("veryfast".to_owned());
                args.push("-pix_fmt".to_owned());
                args.push("yuv420p".to_owned());
            }
        }
        args.push("-c:a".to_owned());
        args.push("aac".to_owned());
        if spec.downmix_stereo {
            args.push("-ac".to_owned());
            args.push("2".to_owned());
        }
        if let Some(bitrate) = spec.max_bitrate {
            args.push("-maxrate".to_owned());
            args.push(bitrate.to_string());
            args.push("-bufsize".to_owned());
            args.push((bitrate * 2).to_string());
        }
    }
    if !spec.copy {
        args.push("-avoid_negative_ts".to_owned());
        args.push("disabled".to_owned());
    }
    args.push("-muxdelay".to_owned());
    args.push("0".to_owned());
    args.push("-f".to_owned());
    args.push("mpegts".to_owned());
    args.push(out_path.to_owned());
    args
}

pub(crate) fn build_subtitle_extract_args(
    video_input: &str,
    source: &SoftSubtitleSource,
    out_vtt: &str,
) -> Vec<String> {
    let mut args = vec!["-y".to_owned(), "-i".to_owned()];
    match source {
        SoftSubtitleSource::Embedded(index) => {
            args.push(video_input.to_owned());
            args.push("-map".to_owned());
            args.push(format!("0:{index}"));
        }
        SoftSubtitleSource::File(path) => {
            args.push(path.clone());
        }
    }
    args.push("-f".to_owned());
    args.push("webvtt".to_owned());
    args.push(out_vtt.to_owned());
    args
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

fn video_filter(spec: &TranscodeSpec, encoder: &VideoEncoder) -> Option<String> {
    let mut filters = Vec::new();
    if let Some(height) = spec.max_height {
        filters.push(format!("scale=-2:{height}"));
    }
    if spec.source_hdr.is_some() {
        filters.push(TONEMAP_TO_SDR.to_owned());
    }
    if let Some(path) = &spec.burn_subtitle_path {
        filters.push(format!(
            "subtitles=filename='{}'",
            escape_subtitle_path(path)
        ));
    }
    if encoder.is_hardware() {
        filters.push("format=nv12".to_owned());
        filters.push("hwupload".to_owned());
    }
    if filters.is_empty() {
        None
    } else {
        Some(filters.join(","))
    }
}

fn escape_subtitle_path(path: &str) -> String {
    path.replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace('\'', "'\\''")
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::media::HdrFormat;
    use domain::session::{
        SegmentContainer, SessionId, StreamGeneration, plan_segments, plan_segments_on_grid,
    };

    fn base_spec() -> TranscodeSpec {
        TranscodeSpec {
            session: SessionId("s1".to_owned()),
            generation: StreamGeneration(1),
            input_path: "/media/movie.mkv".to_owned(),
            duration_ms: 120_000,
            copy: false,
            container: SegmentContainer::MpegTs,
            seek_ms: None,
            audio_track: None,
            max_height: None,
            max_bitrate: None,
            burn_subtitle_path: None,
            soft_subtitle: None,
            downmix_stereo: false,
            source_hdr: None,
        }
    }

    fn segment() -> Segment {
        Segment {
            start_ms: 8_000,
            duration_ms: 4_000,
        }
    }

    fn args_for(spec: &TranscodeSpec) -> Vec<String> {
        build_segment_args(
            spec,
            &segment(),
            "/cache/s1/v0/seg_00002.ts",
            &VideoEncoder::Software,
        )
    }

    fn vaapi_args_for(spec: &TranscodeSpec) -> Vec<String> {
        build_segment_args(
            spec,
            &segment(),
            "/cache/s1/v0/seg_00002.ts",
            &VideoEncoder::Vaapi {
                device: "/dev/dri/renderD128".to_owned(),
            },
        )
    }

    fn pair_after(args: &[String], flag: &str) -> Option<String> {
        args.iter()
            .position(|a| a == flag)
            .map(|i| args[i + 1].clone())
    }

    #[test]
    fn segment_base_seeks_and_writes_mpegts() {
        let args = args_for(&base_spec());
        assert_eq!(args.first().map(String::as_str), Some("-y"));
        assert_eq!(pair_after(&args, "-ss").as_deref(), Some("8.000"));
        assert_eq!(pair_after(&args, "-i").as_deref(), Some("/media/movie.mkv"));
        assert_eq!(pair_after(&args, "-to").as_deref(), Some("11.999"));
        assert_eq!(pair_after(&args, "-f").as_deref(), Some("mpegts"));
        assert_eq!(pair_after(&args, "-c:v").as_deref(), Some("libx264"));
        assert_eq!(pair_after(&args, "-c:a").as_deref(), Some("aac"));
        assert_eq!(pair_after(&args, "-pix_fmt").as_deref(), Some("yuv420p"));
        assert_eq!(
            args.last().map(String::as_str),
            Some("/cache/s1/v0/seg_00002.ts")
        );
        assert!(!args.iter().any(|a| a == "-force_key_frames"));
        assert!(!args.iter().any(|a| a == "-map"));
        assert!(!args.iter().any(|a| a == "-vf"));
        assert!(!args.iter().any(|a| a == "-maxrate"));
        assert!(!args.iter().any(|a| a == "-noaccurate_seek"));
        assert!(!args.iter().any(|a| a == "-t"));
        assert!(!args.iter().any(|a| a == "-output_ts_offset"));
    }

    #[test]
    fn every_segment_shares_one_timeline_so_boundaries_cannot_overlap() {
        // Segments are produced by independent ffmpeg runs. Re-anchoring each one
        // to its own zero let avoid_negative_ts shift only the first segment, by
        // the B-frame reorder delay, so it overran the second by a frame.
        for spec in [
            base_spec(),
            TranscodeSpec {
                copy: true,
                ..base_spec()
            },
        ] {
            let args = args_for(&spec);
            let input = args.iter().position(|a| a == "-i").unwrap();
            let copyts = args.iter().position(|a| a == "-copyts").unwrap();
            assert!(copyts < input, "-copyts has to be an input option");
            assert_eq!(pair_after(&args, "-ss").as_deref(), Some("8.000"));
            assert!(!args.iter().any(|a| a == "-output_ts_offset"));
            assert!(!args.iter().any(|a| a == "-t"));
        }
    }

    #[test]
    fn a_re_encoded_segment_stops_a_millisecond_before_the_next_one_starts() {
        // Plan boundaries are whole milliseconds and frames are not, so a frame
        // sitting just under a rounded-up boundary was emitted by both neighbours.
        let args = args_for(&base_spec());
        assert_eq!(pair_after(&args, "-to").as_deref(), Some("11.999"));
        assert_eq!(
            pair_after(&args, "-avoid_negative_ts").as_deref(),
            Some("disabled")
        );

        let copied = TranscodeSpec {
            copy: true,
            ..base_spec()
        };
        let args = args_for(&copied);
        assert_eq!(
            pair_after(&args, "-to").as_deref(),
            Some("12.000"),
            "a stream copy cuts on keyframes, so it keeps the exact boundary"
        );
        assert!(!args.iter().any(|a| a == "-avoid_negative_ts"));
    }

    #[test]
    fn segment_copy_streams_without_reencode_or_filters() {
        let spec = TranscodeSpec {
            copy: true,
            max_height: Some(720),
            max_bitrate: Some(4_000_000),
            ..base_spec()
        };
        let args = args_for(&spec);
        assert_eq!(pair_after(&args, "-c").as_deref(), Some("copy"));
        assert!(!args.iter().any(|a| a == "-c:v"));
        assert!(!args.iter().any(|a| a == "-c:a"));
        assert!(!args.iter().any(|a| a == "-vf"));
        assert!(!args.iter().any(|a| a == "-maxrate"));
        assert_eq!(pair_after(&args, "-f").as_deref(), Some("mpegts"));
    }

    #[test]
    fn segment_copy_starts_on_the_keyframe_and_keeps_source_timestamps() {
        let spec = TranscodeSpec {
            copy: true,
            ..base_spec()
        };
        let args = args_for(&spec);
        let input = args.iter().position(|a| a == "-i").unwrap();
        let noaccurate = args.iter().position(|a| a == "-noaccurate_seek").unwrap();
        let copyts = args.iter().position(|a| a == "-copyts").unwrap();
        assert!(noaccurate < input);
        assert!(copyts < input);
        assert_eq!(pair_after(&args, "-ss").as_deref(), Some("8.000"));
        assert_eq!(pair_after(&args, "-to").as_deref(), Some("12.000"));
        assert!(args.iter().position(|a| a == "-to").unwrap() > input);
        assert!(!args.iter().any(|a| a == "-t"));
        assert!(!args.iter().any(|a| a == "-output_ts_offset"));
    }

    #[test]
    fn segment_audio_track_maps_video_and_audio() {
        let spec = TranscodeSpec {
            audio_track: Some(2),
            ..base_spec()
        };
        let args = args_for(&spec);
        let maps: Vec<&String> = args
            .iter()
            .enumerate()
            .filter(|(i, a)| *a == "-map" && i + 1 < args.len())
            .map(|(i, _)| &args[i + 1])
            .collect();
        assert_eq!(maps, vec!["0:v:0", "0:2"]);
    }

    #[test]
    fn segment_max_height_adds_scale_filter() {
        let spec = TranscodeSpec {
            max_height: Some(720),
            ..base_spec()
        };
        assert_eq!(
            pair_after(&args_for(&spec), "-vf").as_deref(),
            Some("scale=-2:720")
        );
    }

    #[test]
    fn segment_burn_adds_subtitles_filter() {
        let spec = TranscodeSpec {
            burn_subtitle_path: Some("/media/movie.srt".to_owned()),
            ..base_spec()
        };
        assert_eq!(
            pair_after(&args_for(&spec), "-vf").as_deref(),
            Some("subtitles=filename='/media/movie.srt'")
        );
    }

    #[test]
    fn segment_burn_and_scale_combine_into_one_filter() {
        let spec = TranscodeSpec {
            burn_subtitle_path: Some("/media/movie.srt".to_owned()),
            max_height: Some(480),
            ..base_spec()
        };
        assert_eq!(
            pair_after(&args_for(&spec), "-vf").as_deref(),
            Some("scale=-2:480,subtitles=filename='/media/movie.srt'")
        );
    }

    #[test]
    fn segment_burn_escapes_hostile_paths() {
        let vf = |name: &str| {
            let spec = TranscodeSpec {
                burn_subtitle_path: Some(name.to_owned()),
                ..base_spec()
            };
            pair_after(&args_for(&spec), "-vf").unwrap()
        };
        assert_eq!(
            vf("/m/Mission: Impossible.srt"),
            "subtitles=filename='/m/Mission\\: Impossible.srt'"
        );
        assert_eq!(
            vf("/m/Bob's tape.srt"),
            "subtitles=filename='/m/Bob'\\''s tape.srt'"
        );
        assert_eq!(vf("/m/a,b.srt"), "subtitles=filename='/m/a,b.srt'");
        assert_eq!(vf("/m/a\\b.srt"), "subtitles=filename='/m/a\\\\b.srt'");
    }

    #[test]
    fn segment_max_bitrate_adds_maxrate_and_bufsize() {
        let spec = TranscodeSpec {
            max_bitrate: Some(4_000_000),
            ..base_spec()
        };
        let args = args_for(&spec);
        assert_eq!(pair_after(&args, "-maxrate").as_deref(), Some("4000000"));
        assert_eq!(pair_after(&args, "-bufsize").as_deref(), Some("8000000"));
    }

    #[test]
    fn segment_vaapi_uses_hardware_encoder_and_upload() {
        let args = vaapi_args_for(&base_spec());
        assert_eq!(
            pair_after(&args, "-vaapi_device").as_deref(),
            Some("/dev/dri/renderD128")
        );
        assert_eq!(pair_after(&args, "-c:v").as_deref(), Some("h264_vaapi"));
        assert_eq!(
            pair_after(&args, "-vf").as_deref(),
            Some("format=nv12,hwupload")
        );
        assert!(!args.iter().any(|a| a == "libx264"));
        assert!(!args.iter().any(|a| a == "-preset"));
        assert!(!args.iter().any(|a| a == "-pix_fmt"));
        let device = args.iter().position(|a| a == "-vaapi_device").unwrap();
        let input = args.iter().position(|a| a == "-i").unwrap();
        assert!(device < input);
    }

    #[test]
    fn segment_vaapi_appends_hwupload_after_scale_and_burn() {
        let spec = TranscodeSpec {
            burn_subtitle_path: Some("/media/movie.srt".to_owned()),
            max_height: Some(480),
            ..base_spec()
        };
        assert_eq!(
            pair_after(&vaapi_args_for(&spec), "-vf").as_deref(),
            Some("scale=-2:480,subtitles=filename='/media/movie.srt',format=nv12,hwupload")
        );
    }

    #[test]
    fn segment_vaapi_copy_skips_device_and_encoder() {
        let spec = TranscodeSpec {
            copy: true,
            ..base_spec()
        };
        let args = vaapi_args_for(&spec);
        assert!(!args.iter().any(|a| a == "-vaapi_device"));
        assert!(!args.iter().any(|a| a == "h264_vaapi"));
        assert_eq!(pair_after(&args, "-c").as_deref(), Some("copy"));
    }

    #[test]
    fn segment_hdr_source_tonemaps_to_sdr() {
        let spec = TranscodeSpec {
            source_hdr: Some(HdrFormat::Hdr10),
            ..base_spec()
        };
        assert_eq!(
            pair_after(&args_for(&spec), "-vf").as_deref(),
            Some(TONEMAP_TO_SDR)
        );
    }

    // The scale comes first so the tonemap works on the smaller picture, and the
    // burn stays after the tonemap so subtitle graphics are not tonemapped too.
    #[test]
    fn segment_scales_before_tonemapping_and_burns_last() {
        let spec = TranscodeSpec {
            source_hdr: Some(HdrFormat::Hlg),
            burn_subtitle_path: Some("/media/movie.srt".to_owned()),
            max_height: Some(480),
            ..base_spec()
        };
        assert_eq!(
            pair_after(&args_for(&spec), "-vf").as_deref(),
            Some(
                format!("scale=-2:480,{TONEMAP_TO_SDR},subtitles=filename='/media/movie.srt'")
                    .as_str()
            )
        );
    }

    #[test]
    fn segment_hdr_vaapi_tonemaps_before_hwupload() {
        let spec = TranscodeSpec {
            source_hdr: Some(HdrFormat::DolbyVision),
            ..base_spec()
        };
        assert_eq!(
            pair_after(&vaapi_args_for(&spec), "-vf").as_deref(),
            Some(format!("{TONEMAP_TO_SDR},format=nv12,hwupload").as_str())
        );
    }

    #[test]
    fn segment_copy_ignores_hdr_source() {
        let spec = TranscodeSpec {
            copy: true,
            source_hdr: Some(HdrFormat::Hdr10Plus),
            ..base_spec()
        };
        let args = args_for(&spec);
        assert_eq!(pair_after(&args, "-c").as_deref(), Some("copy"));
        assert!(!args.iter().any(|a| a == "-vf"));
    }

    #[test]
    fn segment_downmix_adds_ac_two() {
        let spec = TranscodeSpec {
            downmix_stereo: true,
            ..base_spec()
        };
        assert_eq!(pair_after(&args_for(&spec), "-ac").as_deref(), Some("2"));
    }

    #[test]
    fn segment_copy_ignores_downmix() {
        let spec = TranscodeSpec {
            copy: true,
            downmix_stereo: true,
            ..base_spec()
        };
        assert!(!args_for(&spec).iter().any(|a| a == "-ac"));
    }

    #[test]
    fn media_playlist_is_vod_with_endlist_and_segments() {
        let plan = plan_segments(&[4_000, 8_000], 10_000, 4_000);
        let playlist = media_playlist(&plan, SegmentContainer::MpegTs, 0);
        assert!(playlist.starts_with("#EXTM3U\n"));
        assert!(playlist.contains("#EXT-X-VERSION:3"));
        assert!(playlist.contains("#EXT-X-PLAYLIST-TYPE:VOD"));
        assert!(playlist.contains("#EXT-X-TARGETDURATION:"));
        assert!(playlist.contains("#EXTINF:4.000,\nseg_00000.ts\n"));
        assert!(playlist.contains("seg_00001.ts"));
        assert!(playlist.contains("seg_00002.ts"));
        assert!(playlist.trim_end().ends_with("#EXT-X-ENDLIST"));
    }

    #[test]
    fn media_playlist_empty_plan_targets_one() {
        let plan = SegmentPlan {
            segments: Vec::new(),
        };
        let playlist = media_playlist(&plan, SegmentContainer::MpegTs, 0);
        assert!(playlist.contains("#EXT-X-TARGETDURATION:1"));
        assert!(playlist.trim_end().ends_with("#EXT-X-ENDLIST"));
    }

    #[test]
    fn an_fmp4_playlist_declares_version_seven_and_maps_the_init_segment() {
        let plan = plan_segments(&[4_000, 8_000], 10_000, 4_000);
        let playlist = media_playlist(&plan, SegmentContainer::Fmp4, 0);
        assert!(playlist.contains("#EXT-X-VERSION:7"));
        assert!(
            playlist.contains("#EXT-X-MAP:URI=\"init.mp4\""),
            "an fmp4 variant is unplayable without the init segment it maps"
        );
        assert!(playlist.contains("#EXTINF:4.000,\nseg_00000.m4s\n"));
        assert!(!playlist.contains(".ts"));
    }

    #[test]
    fn an_fmp4_playlist_maps_the_init_before_the_first_segment() {
        let plan = plan_segments(&[4_000], 8_000, 4_000);
        let playlist = media_playlist(&plan, SegmentContainer::Fmp4, 0);
        let map = playlist
            .find("#EXT-X-MAP")
            .expect("the map line is present");
        let first = playlist
            .find("seg_00000.m4s")
            .expect("segment zero is listed");
        assert!(
            map < first,
            "the map applies only to segments that follow it"
        );
    }

    #[test]
    fn segment_file_name_is_zero_padded_and_matches_the_container() {
        assert_eq!(
            segment_file_name(0, SegmentContainer::MpegTs),
            "seg_00000.ts"
        );
        assert_eq!(
            segment_file_name(42, SegmentContainer::MpegTs),
            "seg_00042.ts"
        );
        assert_eq!(
            segment_file_name(0, SegmentContainer::Fmp4),
            "seg_00000.m4s"
        );
        assert_eq!(
            segment_file_name(42, SegmentContainer::Fmp4),
            "seg_00042.m4s"
        );
    }

    #[test]
    fn the_producer_copies_into_fmp4_and_names_segments_like_the_playlist() {
        let spec = TranscodeSpec {
            copy: true,
            container: SegmentContainer::Fmp4,
            ..base_spec()
        };
        let args = build_producer_args(&spec, "/cache/s1/v0", 0, 0, DEFAULT_READ_RATE);
        assert_eq!(pair_after(&args, "-c").as_deref(), Some("copy"));
        assert_eq!(pair_after(&args, "-f").as_deref(), Some("hls"));
        assert_eq!(
            pair_after(&args, "-hls_segment_type").as_deref(),
            Some("fmp4")
        );
        assert_eq!(
            pair_after(&args, "-hls_fmp4_init_filename").as_deref(),
            Some("init.mp4")
        );
        assert_eq!(
            pair_after(&args, "-hls_segment_filename").as_deref(),
            Some("/cache/s1/v0/seg_%05d.m4s"),
            "ffmpeg must write the exact names our playlist already lists"
        );
        assert_eq!(
            pair_after(&args, "-hls_flags").as_deref(),
            Some("temp_file"),
            "without an atomic rename a partly written segment would be served as complete"
        );
        assert_eq!(pair_after(&args, "-hls_time").as_deref(), Some("4"));
        assert_eq!(
            pair_after(&args, "-hls_playlist_type").as_deref(),
            Some("vod")
        );
    }

    #[test]
    fn the_producer_is_throttled_so_one_session_cannot_write_the_whole_file_at_once() {
        let spec = TranscodeSpec {
            copy: true,
            container: SegmentContainer::Fmp4,
            ..base_spec()
        };
        let args = build_producer_args(&spec, "/cache/s1/v0", 0, 0, DEFAULT_READ_RATE);
        let readrate = pair_after(&args, "-readrate").expect("the producer is rate limited");
        assert!(readrate.parse::<f64>().expect("a numeric rate") > 1.0);
        let rate_at = args.iter().position(|a| a == "-readrate").unwrap();
        let input_at = args.iter().position(|a| a == "-i").unwrap();
        assert!(
            rate_at < input_at,
            "readrate is an input option and is ignored after -i"
        );
    }

    #[test]
    fn a_read_rate_of_zero_lets_the_producer_run_flat_out() {
        let spec = TranscodeSpec {
            copy: true,
            container: SegmentContainer::Fmp4,
            ..base_spec()
        };
        let args = build_producer_args(&spec, "/cache/s1/v0", 0, 0, 0.0);
        assert!(
            !args.iter().any(|a| a == "-readrate"),
            "ffmpeg has no argument for unlimited, so the throttle is left off entirely"
        );
    }

    #[test]
    fn an_admin_read_rate_reaches_ffmpeg() {
        let spec = TranscodeSpec {
            copy: true,
            container: SegmentContainer::Fmp4,
            ..base_spec()
        };
        let args = build_producer_args(&spec, "/cache/s1/v0", 0, 0, 2.5);
        assert_eq!(pair_after(&args, "-readrate").as_deref(), Some("2.5"));
    }

    #[test]
    fn the_producer_writes_its_own_playlist_not_the_one_we_serve() {
        let spec = TranscodeSpec {
            copy: true,
            container: SegmentContainer::Fmp4,
            ..base_spec()
        };
        let args = build_producer_args(&spec, "/cache/s1/v0", 0, 0, DEFAULT_READ_RATE);
        let out = args.last().expect("the output path is last");
        assert_eq!(out, "/cache/s1/v0/producer.m3u8");
        assert!(
            !out.ends_with("index.m3u8"),
            "ffmpeg's playlist grows as it produces; ours is complete from the start"
        );
    }

    #[test]
    fn a_producer_restarted_at_a_seek_numbers_segments_from_there() {
        let spec = TranscodeSpec {
            copy: true,
            container: SegmentContainer::Fmp4,
            ..base_spec()
        };
        let args = build_producer_args(&spec, "/cache/s1/v0", 8_384, 2, DEFAULT_READ_RATE);
        assert_eq!(
            pair_after(&args, "-ss").as_deref(),
            Some("8.384"),
            "the producer resumes at the segment boundary, not at the raw seek position"
        );
        assert_eq!(
            pair_after(&args, "-start_number").as_deref(),
            Some("2"),
            "segment names stay absolute so a restarted run lines up with the playlist"
        );
        let seek_at = args.iter().position(|a| a == "-ss").unwrap();
        let input_at = args.iter().position(|a| a == "-i").unwrap();
        assert!(
            seek_at < input_at,
            "input seeking is what makes a restart fast; after -i it would decode from the start"
        );
    }

    #[test]
    fn a_producer_starting_at_the_beginning_does_not_seek() {
        let spec = TranscodeSpec {
            copy: true,
            container: SegmentContainer::Fmp4,
            ..base_spec()
        };
        let args = build_producer_args(&spec, "/cache/s1/v0", 0, 0, DEFAULT_READ_RATE);
        assert!(!args.iter().any(|a| a == "-ss"));
        assert_eq!(pair_after(&args, "-start_number").as_deref(), Some("0"));
    }

    #[test]
    fn a_tail_playlist_names_segments_absolutely_and_sets_the_media_sequence() {
        let full = plan_segments_on_grid(&[0, 4_950, 8_384, 14_166], 20_000, 4_000);
        let playlist = media_playlist(&full.from_index(2), SegmentContainer::Fmp4, 2);
        assert!(playlist.contains("#EXT-X-MEDIA-SEQUENCE:2"));
        assert!(
            playlist.contains("seg_00002.m4s"),
            "the first entry of a restarted playlist is still segment two"
        );
        assert!(
            !playlist.contains("seg_00000.m4s") && !playlist.contains("seg_00001.m4s"),
            "listing segments the producer will never write makes the demuxer fail to open them"
        );
        assert!(playlist.contains("#EXT-X-MAP:URI=\"init.mp4\""));
    }

    #[test]
    fn the_producer_maps_the_requested_audio_track() {
        let spec = TranscodeSpec {
            copy: true,
            container: SegmentContainer::Fmp4,
            audio_track: Some(3),
            ..base_spec()
        };
        let args = build_producer_args(&spec, "/cache/s1/v0", 0, 0, DEFAULT_READ_RATE);
        assert!(args.windows(2).any(|w| w[0] == "-map" && w[1] == "0:v:0"));
        assert!(args.windows(2).any(|w| w[0] == "-map" && w[1] == "0:3"));
    }

    #[test]
    fn subtitle_extract_embedded_maps_stream() {
        let args = build_subtitle_extract_args(
            "/media/movie.mkv",
            &SoftSubtitleSource::Embedded(3),
            "/cache/s1/subs/subs.vtt",
        );
        assert_eq!(
            args,
            vec![
                "-y",
                "-i",
                "/media/movie.mkv",
                "-map",
                "0:3",
                "-f",
                "webvtt",
                "/cache/s1/subs/subs.vtt",
            ]
        );
    }

    #[test]
    fn subtitle_extract_file_reads_sidecar() {
        let args = build_subtitle_extract_args(
            "/media/movie.mkv",
            &SoftSubtitleSource::File("/media/movie.en.srt".to_owned()),
            "/cache/s1/subs/subs.vtt",
        );
        assert_eq!(
            args,
            vec![
                "-y",
                "-i",
                "/media/movie.en.srt",
                "-f",
                "webvtt",
                "/cache/s1/subs/subs.vtt",
            ]
        );
        assert!(!args.iter().any(|a| a == "-map"));
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

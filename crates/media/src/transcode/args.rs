use std::path::Path;

use domain::session::TranscodeSpec;

const HLS_SEGMENT_SECONDS: &str = "4";
const SEGMENT_PATTERN: &str = "seg_%05d.ts";
const PLAYLIST_NAME: &str = "index.m3u8";

pub(crate) fn build_hls_args(spec: &TranscodeSpec, output_dir: &Path) -> Vec<String> {
    let mut args = vec!["-y".to_owned()];
    if let Some(ms) = spec.seek_ms {
        args.push("-ss".to_owned());
        args.push(format!("{:.3}", ms as f64 / 1000.0));
    }
    args.push("-i".to_owned());
    args.push(spec.input_path.clone());
    if let Some(idx) = spec.audio_track {
        args.push("-map".to_owned());
        args.push("0:v:0".to_owned());
        args.push("-map".to_owned());
        args.push(format!("0:{idx}"));
    }
    if let Some(filter) = video_filter(spec) {
        args.push("-vf".to_owned());
        args.push(filter);
    }
    if let Some(bitrate) = spec.max_bitrate {
        args.push("-maxrate".to_owned());
        args.push(bitrate.to_string());
        args.push("-bufsize".to_owned());
        args.push((bitrate * 2).to_string());
    }
    args.push("-force_key_frames".to_owned());
    args.push(format!("expr:gte(t,n_forced*{HLS_SEGMENT_SECONDS})"));
    args.push("-f".to_owned());
    args.push("hls".to_owned());
    args.push("-hls_time".to_owned());
    args.push(HLS_SEGMENT_SECONDS.to_owned());
    args.push("-hls_list_size".to_owned());
    args.push("0".to_owned());
    args.push("-hls_playlist_type".to_owned());
    args.push("event".to_owned());
    let variant_dir = output_dir.join(crate::hls::VARIANT);
    args.push("-hls_segment_filename".to_owned());
    args.push(path_in(&variant_dir, SEGMENT_PATTERN));
    args.push(path_in(&variant_dir, PLAYLIST_NAME));
    args
}

fn video_filter(spec: &TranscodeSpec) -> Option<String> {
    let mut filters = Vec::new();
    if let Some(path) = &spec.burn_subtitle_path {
        filters.push(format!(
            "subtitles=filename='{}'",
            escape_subtitle_path(path)
        ));
    }
    if let Some(height) = spec.max_height {
        filters.push(format!("scale=-2:{height}"));
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

fn path_in(dir: &Path, name: &str) -> String {
    dir.join(name).to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::session::SessionId;

    fn base_spec() -> TranscodeSpec {
        TranscodeSpec {
            session: SessionId("s1".to_owned()),
            input_path: "/media/movie.mkv".to_owned(),
            seek_ms: None,
            audio_track: None,
            max_height: None,
            max_bitrate: None,
            burn_subtitle_path: None,
        }
    }

    fn args_for(spec: &TranscodeSpec) -> Vec<String> {
        build_hls_args(spec, Path::new("/cache/s1"))
    }

    fn pair_after(args: &[String], flag: &str) -> Option<String> {
        args.iter()
            .position(|a| a == flag)
            .map(|i| args[i + 1].clone())
    }

    #[test]
    fn base_has_input_and_hls_output_only() {
        let args = args_for(&base_spec());
        assert_eq!(args.first().map(String::as_str), Some("-y"));
        assert_eq!(pair_after(&args, "-i").as_deref(), Some("/media/movie.mkv"));
        assert_eq!(pair_after(&args, "-f").as_deref(), Some("hls"));
        assert_eq!(pair_after(&args, "-hls_time").as_deref(), Some("4"));
        assert_eq!(pair_after(&args, "-hls_list_size").as_deref(), Some("0"));
        assert_eq!(
            pair_after(&args, "-hls_playlist_type").as_deref(),
            Some("event")
        );
        assert_eq!(
            pair_after(&args, "-force_key_frames").as_deref(),
            Some("expr:gte(t,n_forced*4)")
        );
        assert_eq!(
            pair_after(&args, "-hls_segment_filename").as_deref(),
            Some("/cache/s1/v0/seg_%05d.ts")
        );
        assert_eq!(
            args.last().map(String::as_str),
            Some("/cache/s1/v0/index.m3u8")
        );
        assert!(!args.iter().any(|a| a == "-ss"));
        assert!(!args.iter().any(|a| a == "-map"));
        assert!(!args.iter().any(|a| a == "-vf"));
        assert!(!args.iter().any(|a| a == "-maxrate"));
    }

    #[test]
    fn seek_adds_ss_in_seconds() {
        let spec = TranscodeSpec {
            seek_ms: Some(12_500),
            ..base_spec()
        };
        assert_eq!(
            pair_after(&args_for(&spec), "-ss").as_deref(),
            Some("12.500")
        );
    }

    #[test]
    fn audio_track_maps_video_and_audio() {
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
    fn max_height_adds_scale_filter() {
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
    fn burn_subtitle_adds_subtitles_filter() {
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
    fn burn_and_scale_combine_into_one_filter() {
        let spec = TranscodeSpec {
            burn_subtitle_path: Some("/media/movie.srt".to_owned()),
            max_height: Some(480),
            ..base_spec()
        };
        assert_eq!(
            pair_after(&args_for(&spec), "-vf").as_deref(),
            Some("subtitles=filename='/media/movie.srt',scale=-2:480")
        );
    }

    #[test]
    fn burn_subtitle_escapes_hostile_paths() {
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
    fn max_bitrate_adds_maxrate_and_bufsize() {
        let spec = TranscodeSpec {
            max_bitrate: Some(4_000_000),
            ..base_spec()
        };
        let args = args_for(&spec);
        assert_eq!(pair_after(&args, "-maxrate").as_deref(), Some("4000000"));
        assert_eq!(pair_after(&args, "-bufsize").as_deref(), Some("8000000"));
    }
}

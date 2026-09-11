use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use domain::error::ProfileError;
use domain::media::HdrFormat;
use domain::profile::{
    AudioCodecCap, CapabilityProfile, Container, ProfileRegistry, VideoCodecCap,
};

const GENERIC: &str = "generic";

const BUILTINS: &[(&str, &str)] = &[
    ("desktop", include_str!("data/desktop.json")),
    ("roku", include_str!("data/roku.json")),
    ("safari", include_str!("data/safari.json")),
    ("chrome", include_str!("data/chrome.json")),
    ("firefox", include_str!("data/firefox.json")),
    ("ios", include_str!("data/ios.json")),
    ("android", include_str!("data/android.json")),
    (GENERIC, include_str!("data/generic.json")),
];

#[derive(Deserialize)]
struct RawProfile {
    containers: Vec<String>,
    video: Vec<RawVideoCodec>,
    audio: Vec<RawAudioCodec>,
    #[serde(default)]
    hdr: Vec<String>,
    max_width: u32,
    max_height: u32,
    max_bitrate: u64,
    #[serde(default)]
    max_frame_rate: Option<u32>,
}

#[derive(Deserialize)]
struct RawVideoCodec {
    codec: String,
    #[serde(default)]
    max_level: Option<String>,
    max_bit_depth: u8,
    #[serde(default = "smooth_unless_stated")]
    smooth: bool,
}

fn smooth_unless_stated() -> bool {
    true
}

#[derive(Deserialize)]
struct RawAudioCodec {
    codec: String,
    max_channels: u8,
}

pub struct BuiltinProfiles {
    profiles: HashMap<String, CapabilityProfile>,
    generic: CapabilityProfile,
}

impl BuiltinProfiles {
    pub fn load() -> Result<Self, ProfileError> {
        Self::load_with_overrides(HashMap::new())
    }

    pub fn load_from_dir(dir: Option<&Path>) -> Result<Self, ProfileError> {
        match dir {
            Some(dir) => Self::load_with_overrides(read_overrides(dir)?),
            None => Self::load(),
        }
    }

    pub fn load_with_overrides(
        overrides: HashMap<String, CapabilityProfile>,
    ) -> Result<Self, ProfileError> {
        let mut profiles = HashMap::new();
        for &(name, json) in BUILTINS {
            profiles.insert(name.to_owned(), parse_profile(json)?);
        }
        for (name, profile) in overrides {
            profile.validate()?;
            profiles.insert(name, profile);
        }
        let generic = profiles[GENERIC].clone();
        Ok(Self { profiles, generic })
    }
}

fn read_overrides(dir: &Path) -> Result<HashMap<String, CapabilityProfile>, ProfileError> {
    let entries = std::fs::read_dir(dir).map_err(|error| unreadable(dir, &error))?;
    let mut overrides = HashMap::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let json = std::fs::read_to_string(&path).map_err(|error| unreadable(&path, &error))?;
        let profile = parse_profile(&json)
            .map_err(|error| ProfileError::Invalid(format!("{}: {error}", path.display())))?;
        overrides.insert(name.to_owned(), profile);
    }
    Ok(overrides)
}

fn unreadable(path: &Path, error: &std::io::Error) -> ProfileError {
    ProfileError::Unreadable(format!("{}: {error}", path.display()))
}

impl ProfileRegistry for BuiltinProfiles {
    fn resolve(&self, platform: &str) -> CapabilityProfile {
        self.profiles.get(platform).cloned().unwrap_or_else(|| self.generic.clone())
    }
}

fn parse_profile(json: &str) -> Result<CapabilityProfile, ProfileError> {
    let raw: RawProfile =
        serde_json::from_str(json).map_err(|e| ProfileError::Parse(e.to_string()))?;
    to_domain(raw)
}

fn to_domain(raw: RawProfile) -> Result<CapabilityProfile, ProfileError> {
    let containers =
        raw.containers.iter().map(|c| map_container(c)).collect::<Result<Vec<_>, _>>()?;
    let hdr = raw.hdr.iter().map(|h| map_hdr(h)).collect::<Result<Vec<_>, _>>()?;
    let video = raw
        .video
        .into_iter()
        .map(|v| VideoCodecCap {
            codec: v.codec,
            max_level: v.max_level,
            max_bit_depth: v.max_bit_depth,
            smooth: v.smooth,
        })
        .collect();
    let audio = raw
        .audio
        .into_iter()
        .map(|a| AudioCodecCap { codec: a.codec, max_channels: a.max_channels })
        .collect();
    let profile = CapabilityProfile {
        containers,
        video,
        audio,
        hdr,
        max_width: raw.max_width,
        max_height: raw.max_height,
        max_bitrate: raw.max_bitrate,
        max_frame_rate: raw.max_frame_rate,
    };
    profile.validate()?;
    Ok(profile)
}

fn map_container(value: &str) -> Result<Container, ProfileError> {
    match value {
        "mp4" => Ok(Container::Mp4),
        "mkv" => Ok(Container::Mkv),
        "ts" => Ok(Container::Ts),
        "hls" => Ok(Container::Hls),
        other => Err(ProfileError::Invalid(format!("unknown container: {other}"))),
    }
}

fn map_hdr(value: &str) -> Result<HdrFormat, ProfileError> {
    HdrFormat::parse(value)
        .ok_or_else(|| ProfileError::Invalid(format!("unknown hdr format: {value}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtins_load_and_validate() {
        let registry = BuiltinProfiles::load().expect("built-ins should load");
        for &(name, _) in BUILTINS {
            let profile = registry.resolve(name);
            assert!(profile.validate().is_ok());
            // A profile file says nothing about smoothness, so every built-in
            // has to keep negotiating exactly as it did before clients could
            // report otherwise.
            assert!(profile.video.iter().all(|codec| codec.smooth));
            assert_eq!(profile.max_frame_rate, None);
        }
    }

    #[test]
    fn resolves_known_platform() {
        let registry = BuiltinProfiles::load().unwrap();
        let roku = registry.resolve("roku");
        assert!(roku.video.iter().any(|v| v.codec == "hevc"));
        assert_eq!(roku.max_height, 2160);
        assert!(roku.hdr.contains(&HdrFormat::Hdr10));
    }

    #[test]
    fn the_desktop_player_direct_plays_hdr_and_ten_bit_hevc() {
        let registry = BuiltinProfiles::load().unwrap();
        let desktop = registry.resolve("desktop");
        // mpv decodes these natively, so anything narrower here makes the
        // server tone map and downscale 4K HDR in real time for nothing.
        let hevc = desktop.video.iter().find(|v| v.codec == "hevc").expect("hevc present");
        assert!(hevc.max_bit_depth >= 10);
        assert_eq!(desktop.max_height, 2160);
        assert!(desktop.hdr.contains(&HdrFormat::Hdr10));
        assert!(desktop.hdr.contains(&HdrFormat::Hlg));
        assert!(desktop.containers.contains(&Container::Mkv));
        assert!(desktop.audio.iter().any(|a| a.codec == "truehd"));
    }

    #[test]
    fn firefox_direct_plays_4k_but_leaves_out_what_it_cannot_decode() {
        let registry = BuiltinProfiles::load().unwrap();
        let firefox = registry.resolve("firefox");
        assert_eq!(firefox.max_height, 2160);
        assert!(firefox.video.iter().any(|v| v.codec == "av1"));
        // Claiming something Firefox cannot decode fails playback outright,
        // which is worse than the transcode leaving it out costs. Firefox has
        // no ac3/dts at all, and its hevc and HDR support is per-platform and
        // hardware dependent.
        assert!(!firefox.audio.iter().any(|a| a.codec == "ac3"));
        assert!(!firefox.audio.iter().any(|a| a.codec == "dts"));
        assert!(!firefox.video.iter().any(|v| v.codec == "hevc"));
        assert!(firefox.hdr.is_empty());
        // Firefox plays WebM rather than Matroska generally, so an mkv source
        // has to remux, which is still a copy rather than a re-encode.
        assert!(!firefox.containers.contains(&Container::Mkv));
    }

    #[test]
    fn unknown_platform_falls_back_to_generic() {
        let registry = BuiltinProfiles::load().unwrap();
        assert_eq!(registry.resolve("nintendo-switch"), registry.resolve(GENERIC));
    }

    #[test]
    fn override_replaces_and_adds() {
        let custom = CapabilityProfile {
            containers: vec![Container::Mp4],
            video: vec![VideoCodecCap {
                codec: "av1".to_owned(),
                max_level: None,
                max_bit_depth: 10,
                smooth: true,
            }],
            audio: vec![AudioCodecCap { codec: "opus".to_owned(), max_channels: 2 }],
            hdr: vec![HdrFormat::Hdr10],
            max_width: 7680,
            max_height: 4320,
            max_bitrate: 100_000_000,
            max_frame_rate: None,
        };
        let mut overrides = HashMap::new();
        overrides.insert("roku".to_owned(), custom.clone());
        overrides.insert("my-tv".to_owned(), custom.clone());

        let registry = BuiltinProfiles::load_with_overrides(overrides).unwrap();
        assert_eq!(registry.resolve("roku"), custom);
        assert_eq!(registry.resolve("my-tv"), custom);
    }

    #[test]
    fn override_validation_rejects_invalid() {
        let mut overrides = HashMap::new();
        overrides.insert(
            "bad".to_owned(),
            CapabilityProfile {
                containers: vec![],
                video: vec![],
                audio: vec![],
                hdr: vec![],
                max_width: 0,
                max_height: 0,
                max_bitrate: 0,
                max_frame_rate: None,
            },
        );
        assert!(matches!(
            BuiltinProfiles::load_with_overrides(overrides),
            Err(ProfileError::Invalid(_))
        ));
    }

    #[test]
    fn empty_video_is_invalid() {
        let json = r#"{"containers":["mp4"],"video":[],"audio":[{"codec":"aac","max_channels":2}],"max_width":1920,"max_height":1080,"max_bitrate":8000000}"#;
        assert!(matches!(parse_profile(json), Err(ProfileError::Invalid(_))));
    }

    #[test]
    fn empty_audio_is_invalid() {
        let json = r#"{"containers":["mp4"],"video":[{"codec":"h264","max_bit_depth":8}],"audio":[],"max_width":1920,"max_height":1080,"max_bitrate":8000000}"#;
        assert!(matches!(parse_profile(json), Err(ProfileError::Invalid(_))));
    }

    #[test]
    fn zero_dimension_is_invalid() {
        let json = r#"{"containers":["mp4"],"video":[{"codec":"h264","max_bit_depth":8}],"audio":[{"codec":"aac","max_channels":2}],"max_width":0,"max_height":1080,"max_bitrate":8000000}"#;
        assert!(matches!(parse_profile(json), Err(ProfileError::Invalid(_))));
    }

    #[test]
    fn unknown_container_is_invalid() {
        let json = r#"{"containers":["flv"],"video":[{"codec":"h264","max_bit_depth":8}],"audio":[{"codec":"aac","max_channels":2}],"max_width":1920,"max_height":1080,"max_bitrate":8000000}"#;
        assert!(matches!(parse_profile(json), Err(ProfileError::Invalid(_))));
    }

    #[test]
    fn unknown_hdr_is_invalid() {
        let json = r#"{"containers":["mp4"],"video":[{"codec":"h264","max_bit_depth":8}],"audio":[{"codec":"aac","max_channels":2}],"hdr":["hdr11"],"max_width":1920,"max_height":1080,"max_bitrate":8000000}"#;
        assert!(matches!(parse_profile(json), Err(ProfileError::Invalid(_))));
    }

    #[test]
    fn malformed_json_is_parse_error() {
        assert!(matches!(parse_profile("not json"), Err(ProfileError::Parse(_))));
    }

    #[test]
    fn a_profile_can_declare_a_codec_it_cannot_decode_smoothly() {
        let json = r#"{"containers":["mp4"],"video":[{"codec":"av1","max_bit_depth":10,"smooth":false}],"audio":[{"codec":"aac","max_channels":2}],"max_width":3840,"max_height":2160,"max_bitrate":40000000,"max_frame_rate":30}"#;
        let profile = parse_profile(json).unwrap();
        assert!(!profile.video[0].smooth);
        assert_eq!(profile.max_frame_rate, Some(30));
    }

    fn write_profile(dir: &std::path::Path, name: &str, body: &str) {
        std::fs::write(dir.join(name), body).unwrap();
    }

    const OVERRIDE: &str = r#"{"containers":["mp4"],"video":[{"codec":"h264","max_bit_depth":8}],"audio":[{"codec":"aac","max_channels":2}],"max_width":1280,"max_height":720,"max_bitrate":4000000}"#;

    #[test]
    fn without_a_directory_only_the_built_ins_load() {
        let registry = BuiltinProfiles::load_from_dir(None).unwrap();
        assert_eq!(registry.resolve("roku"), BuiltinProfiles::load().unwrap().resolve("roku"));
    }

    #[test]
    fn a_file_replaces_the_built_in_it_names_and_adds_the_ones_it_does_not() {
        // The operator escape hatch for a client that cannot measure: a Roku
        // Express cannot do what roku.json claims for every model.
        let dir = tempfile::tempdir().unwrap();
        write_profile(dir.path(), "roku.json", OVERRIDE);
        write_profile(dir.path(), "lounge-tv.json", OVERRIDE);
        write_profile(dir.path(), "notes.txt", "ignored");

        let registry = BuiltinProfiles::load_from_dir(Some(dir.path())).unwrap();
        assert_eq!(registry.resolve("roku").max_height, 720);
        assert_eq!(registry.resolve("lounge-tv").max_height, 720);
        assert_eq!(registry.resolve("android").max_height, 2160);
        assert_eq!(registry.resolve("notes"), registry.resolve(GENERIC));
    }

    #[test]
    fn a_missing_directory_refuses_to_start() {
        // A silently ignored override is worse than a refusal to boot: the
        // operator would be reading a profile they think they replaced.
        let error = BuiltinProfiles::load_from_dir(Some(std::path::Path::new("/no/such/dir")))
            .err()
            .expect("a missing directory should not be ignored");
        assert!(matches!(error, ProfileError::Unreadable(_)));
        assert!(error.to_string().contains("/no/such/dir"));
    }

    #[test]
    fn an_unreadable_file_refuses_to_start() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("roku.json")).unwrap();
        assert!(matches!(
            BuiltinProfiles::load_from_dir(Some(dir.path())),
            Err(ProfileError::Unreadable(_))
        ));
    }

    #[test]
    fn a_broken_file_names_itself() {
        let dir = tempfile::tempdir().unwrap();
        write_profile(dir.path(), "roku.json", "not json");
        let error = BuiltinProfiles::load_from_dir(Some(dir.path()))
            .err()
            .expect("a broken override should not be ignored");
        assert!(error.to_string().contains("roku.json"));
    }
}

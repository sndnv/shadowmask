use std::collections::HashMap;

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
}

#[derive(Deserialize)]
struct RawVideoCodec {
    codec: String,
    #[serde(default)]
    max_level: Option<String>,
    max_bit_depth: u8,
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

    pub fn load_with_overrides(
        overrides: HashMap<String, CapabilityProfile>,
    ) -> Result<Self, ProfileError> {
        let mut profiles = HashMap::new();
        for &(name, json) in BUILTINS {
            profiles.insert(name.to_owned(), parse_profile(json)?);
        }
        for (name, profile) in overrides {
            validate(&profile)?;
            profiles.insert(name, profile);
        }
        let generic = profiles[GENERIC].clone();
        Ok(Self { profiles, generic })
    }
}

impl ProfileRegistry for BuiltinProfiles {
    fn resolve(&self, platform: &str) -> CapabilityProfile {
        self.profiles
            .get(platform)
            .cloned()
            .unwrap_or_else(|| self.generic.clone())
    }
}

fn parse_profile(json: &str) -> Result<CapabilityProfile, ProfileError> {
    let raw: RawProfile =
        serde_json::from_str(json).map_err(|e| ProfileError::Parse(e.to_string()))?;
    to_domain(raw)
}

fn to_domain(raw: RawProfile) -> Result<CapabilityProfile, ProfileError> {
    let containers = raw
        .containers
        .iter()
        .map(|c| map_container(c))
        .collect::<Result<Vec<_>, _>>()?;
    let hdr = raw
        .hdr
        .iter()
        .map(|h| map_hdr(h))
        .collect::<Result<Vec<_>, _>>()?;
    let video = raw
        .video
        .into_iter()
        .map(|v| VideoCodecCap {
            codec: v.codec,
            max_level: v.max_level,
            max_bit_depth: v.max_bit_depth,
        })
        .collect();
    let audio = raw
        .audio
        .into_iter()
        .map(|a| AudioCodecCap {
            codec: a.codec,
            max_channels: a.max_channels,
        })
        .collect();
    let profile = CapabilityProfile {
        containers,
        video,
        audio,
        hdr,
        max_width: raw.max_width,
        max_height: raw.max_height,
        max_bitrate: raw.max_bitrate,
    };
    validate(&profile)?;
    Ok(profile)
}

fn validate(p: &CapabilityProfile) -> Result<(), ProfileError> {
    if p.containers.is_empty() {
        return Err(ProfileError::Invalid("no containers".to_owned()));
    }
    if p.video.is_empty() {
        return Err(ProfileError::Invalid("no video codecs".to_owned()));
    }
    if p.audio.is_empty() {
        return Err(ProfileError::Invalid("no audio codecs".to_owned()));
    }
    if p.max_width == 0 || p.max_height == 0 || p.max_bitrate == 0 {
        return Err(ProfileError::Invalid("non-positive limits".to_owned()));
    }
    Ok(())
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
    match value {
        "hdr10" => Ok(HdrFormat::Hdr10),
        "hdr10plus" => Ok(HdrFormat::Hdr10Plus),
        "dolby_vision" => Ok(HdrFormat::DolbyVision),
        "hlg" => Ok(HdrFormat::Hlg),
        other => Err(ProfileError::Invalid(format!(
            "unknown hdr format: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtins_load_and_validate() {
        let registry = BuiltinProfiles::load().expect("built-ins should load");
        for &(name, _) in BUILTINS {
            let profile = registry.resolve(name);
            assert!(validate(&profile).is_ok());
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
        let hevc = desktop
            .video
            .iter()
            .find(|v| v.codec == "hevc")
            .expect("hevc present");
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
        assert_eq!(
            registry.resolve("nintendo-switch"),
            registry.resolve(GENERIC)
        );
    }

    #[test]
    fn override_replaces_and_adds() {
        let custom = CapabilityProfile {
            containers: vec![Container::Mp4],
            video: vec![VideoCodecCap {
                codec: "av1".to_owned(),
                max_level: None,
                max_bit_depth: 10,
            }],
            audio: vec![AudioCodecCap {
                codec: "opus".to_owned(),
                max_channels: 2,
            }],
            hdr: vec![HdrFormat::Hdr10],
            max_width: 7680,
            max_height: 4320,
            max_bitrate: 100_000_000,
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
        assert!(matches!(
            parse_profile("not json"),
            Err(ProfileError::Parse(_))
        ));
    }
}

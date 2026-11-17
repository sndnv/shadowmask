use jiff::Timestamp;
use serde::Serialize;

use domain::session::PROFILE_VERSION;

use super::{Capability, FontSet, RatingSystem};

const FEATURES: &[&str] = &["hls", "trickplay", "transcoding", "link_codes", "api_tokens"];

#[derive(Debug, Serialize)]
pub struct ServerInfoResponse {
    pub version: String,
    pub features: Vec<String>,
    pub capabilities: Vec<Capability>,
    pub profile_version: u32,
    pub rating_systems: Vec<RatingSystem>,
    pub fonts: Option<FontSet>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_ms: Option<u64>,
}

impl ServerInfoResponse {
    pub fn current(capabilities: Vec<Capability>, fonts: Option<FontSet>) -> Self {
        ServerInfoResponse {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            features: FEATURES.iter().map(|f| (*f).to_owned()).collect(),
            capabilities,
            profile_version: PROFILE_VERSION,
            rating_systems: RatingSystem::known(),
            fonts,
            started_at: None,
            uptime_ms: None,
        }
    }

    pub fn with_uptime(self, started_at: Timestamp, now: Timestamp) -> Self {
        let uptime_ms = u64::try_from(now.as_millisecond() - started_at.as_millisecond());
        Self {
            started_at: Some(started_at.to_string()),
            uptime_ms: Some(uptime_ms.unwrap_or(0)),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::FontFile;
    use super::*;

    fn font(name: &str) -> FontFile {
        FontFile { url: format!("/fonts/{name}"), size: 3, sha256: "abc".to_owned() }
    }

    #[test]
    fn serializes_the_fonts_or_null_without_them() {
        let fonts = FontSet {
            name: "Noto Sans CJK SC".to_owned(),
            regular: font("Regular.otf"),
            medium: font("Medium.otf"),
            license: font("LICENSE"),
            italic: Some(font("Italic.ttf")),
            italic_license: Some(font("LICENSE-Italic")),
        };
        let upright = FontSet { italic: None, italic_license: None, ..fonts.clone() };

        let value =
            serde_json::to_value(ServerInfoResponse::current(Vec::new(), Some(fonts))).unwrap();
        assert_eq!(value["fonts"]["name"], "Noto Sans CJK SC");
        assert_eq!(value["fonts"]["regular"]["url"], "/fonts/Regular.otf");
        assert_eq!(value["fonts"]["medium"]["size"], 3);
        assert_eq!(value["fonts"]["license"]["sha256"], "abc");
        assert_eq!(value["fonts"]["italic"]["url"], "/fonts/Italic.ttf");
        assert_eq!(value["fonts"]["italic_license"]["url"], "/fonts/LICENSE-Italic");

        let value =
            serde_json::to_value(ServerInfoResponse::current(Vec::new(), Some(upright))).unwrap();
        assert!(value["fonts"]["italic"].is_null());
        assert!(value["fonts"]["italic_license"].is_null());

        let none = serde_json::to_value(ServerInfoResponse::current(Vec::new(), None)).unwrap();
        assert!(none["fonts"].is_null());
    }

    #[test]
    fn leaves_the_uptime_out_unless_it_is_given() {
        let value = serde_json::to_value(ServerInfoResponse::current(Vec::new(), None)).unwrap();
        assert!(value.get("started_at").is_none(), "{value}");
        assert!(value.get("uptime_ms").is_none(), "{value}");
    }

    #[test]
    fn serializes_the_start_and_the_time_since() {
        let started = Timestamp::from_second(1_700_000_000).unwrap();
        let now = Timestamp::from_millisecond(1_700_090_061_500).unwrap();
        let info = ServerInfoResponse::current(Vec::new(), None).with_uptime(started, now);

        let value = serde_json::to_value(info).unwrap();
        assert_eq!(value["started_at"], "2023-11-14T22:13:20Z");
        assert_eq!(value["uptime_ms"], 90_061_500);
    }

    #[test]
    fn a_start_after_now_is_no_uptime() {
        let started = Timestamp::from_second(1_700_000_001).unwrap();
        let now = Timestamp::from_second(1_700_000_000).unwrap();
        let info = ServerInfoResponse::current(Vec::new(), None).with_uptime(started, now);

        assert_eq!(serde_json::to_value(info).unwrap()["uptime_ms"], 0);
    }

    #[test]
    fn serializes_descriptor() {
        let value = serde_json::to_value(ServerInfoResponse::current(
            vec![Capability::new("upscaling", true, true), Capability::new("tmdb", true, false)],
            None,
        ))
        .unwrap();
        assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(value["profile_version"], 1);
        let features = value["features"].as_array().unwrap();
        assert!(features.iter().any(|f| f == "hls"));
        assert!(features.iter().any(|f| f == "transcoding"));
        let capabilities = value["capabilities"].as_array().unwrap();
        assert!(capabilities.iter().any(|c| {
            c["name"] == "upscaling" && c["available"] == true && c["enabled"] == true
        }));
        assert!(capabilities.iter().any(|c| { c["name"] == "tmdb" && c["enabled"] == false }));
        let systems = value["rating_systems"].as_array().unwrap();
        assert!(systems.iter().any(|s| s["system"] == "bbfc"));
    }
}

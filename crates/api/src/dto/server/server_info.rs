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
        };

        let value =
            serde_json::to_value(ServerInfoResponse::current(Vec::new(), Some(fonts))).unwrap();
        assert_eq!(value["fonts"]["name"], "Noto Sans CJK SC");
        assert_eq!(value["fonts"]["regular"]["url"], "/fonts/Regular.otf");
        assert_eq!(value["fonts"]["medium"]["size"], 3);
        assert_eq!(value["fonts"]["license"]["sha256"], "abc");

        let none = serde_json::to_value(ServerInfoResponse::current(Vec::new(), None)).unwrap();
        assert!(none["fonts"].is_null());
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

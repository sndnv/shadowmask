use serde::Serialize;

use domain::session::PROFILE_VERSION;

use super::{Capability, RatingSystem};

const FEATURES: &[&str] = &[
    "hls",
    "trickplay",
    "transcoding",
    "link_codes",
    "api_tokens",
];

#[derive(Debug, Serialize)]
pub struct ServerInfoResponse {
    pub version: String,
    pub features: Vec<String>,
    pub capabilities: Vec<Capability>,
    pub profile_version: u32,
    pub rating_systems: Vec<RatingSystem>,
}

impl ServerInfoResponse {
    pub fn current(capabilities: Vec<Capability>) -> Self {
        ServerInfoResponse {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            features: FEATURES.iter().map(|f| (*f).to_owned()).collect(),
            capabilities,
            profile_version: PROFILE_VERSION,
            rating_systems: RatingSystem::known(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_descriptor() {
        let value = serde_json::to_value(ServerInfoResponse::current(vec![
            Capability::new("upscaling", true, true),
            Capability::new("tmdb", true, false),
        ]))
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
        assert!(
            capabilities
                .iter()
                .any(|c| { c["name"] == "tmdb" && c["enabled"] == false })
        );
        let systems = value["rating_systems"].as_array().unwrap();
        assert!(systems.iter().any(|s| s["system"] == "bbfc"));
    }
}

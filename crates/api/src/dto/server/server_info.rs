use serde::Serialize;

use domain::session::PROFILE_VERSION;

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
    pub profile_version: u32,
}

impl ServerInfoResponse {
    pub fn current() -> Self {
        ServerInfoResponse {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            features: FEATURES.iter().map(|f| (*f).to_owned()).collect(),
            profile_version: PROFILE_VERSION,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_descriptor() {
        let value = serde_json::to_value(ServerInfoResponse::current()).unwrap();
        assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(value["profile_version"], 1);
        let features = value["features"].as_array().unwrap();
        assert!(features.iter().any(|f| f == "hls"));
        assert!(features.iter().any(|f| f == "transcoding"));
    }
}

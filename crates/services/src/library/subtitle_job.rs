use domain::catalog::VersionId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleJobPayload {
    pub version_id: VersionId,
    pub imdb_id: Option<String>,
    pub title: Option<String>,
    pub languages: Vec<String>,
    pub season: Option<u16>,
    pub episode: Option<u16>,
}

impl SubtitleJobPayload {
    pub fn encode(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&Wire::from(self))
    }

    pub fn decode(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str::<Wire>(raw).map(Self::from)
    }
}

#[derive(Serialize, Deserialize)]
struct Wire {
    version_id: String,
    imdb_id: Option<String>,
    title: Option<String>,
    languages: Vec<String>,
    season: Option<u16>,
    episode: Option<u16>,
}

impl From<&SubtitleJobPayload> for Wire {
    fn from(payload: &SubtitleJobPayload) -> Self {
        Wire {
            version_id: payload.version_id.0.clone(),
            imdb_id: payload.imdb_id.clone(),
            title: payload.title.clone(),
            languages: payload.languages.clone(),
            season: payload.season,
            episode: payload.episode,
        }
    }
}

impl From<Wire> for SubtitleJobPayload {
    fn from(wire: Wire) -> Self {
        SubtitleJobPayload {
            version_id: VersionId(wire.version_id),
            imdb_id: wire.imdb_id,
            title: wire.title,
            languages: wire.languages,
            season: wire.season,
            episode: wire.episode,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let payload = SubtitleJobPayload {
            version_id: VersionId("v1".into()),
            imdb_id: Some("tt0133093".into()),
            title: Some("The Matrix".into()),
            languages: vec!["en".into(), "es".into()],
            season: Some(1),
            episode: Some(2),
        };
        let encoded = payload.encode().unwrap();
        assert_eq!(SubtitleJobPayload::decode(&encoded).unwrap(), payload);
    }

    #[test]
    fn decode_rejects_malformed_json() {
        assert!(SubtitleJobPayload::decode("not json").is_err());
    }
}

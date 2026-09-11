use domain::catalog::VersionId;
use serde::{Deserialize, Serialize};

use crate::job::encode_payload;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleJobPayload {
    pub version_id: VersionId,
    pub imdb_id: Option<String>,
    pub title: Option<String>,
    pub languages: Vec<String>,
    pub season: Option<u16>,
    pub episode: Option<u16>,
    pub transcribe_on_miss: bool,
}

impl SubtitleJobPayload {
    pub fn encode(&self) -> String {
        encode_payload(&Wire::from(self))
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
    #[serde(default)]
    transcribe_on_miss: bool,
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
            transcribe_on_miss: payload.transcribe_on_miss,
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
            transcribe_on_miss: wire.transcribe_on_miss,
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
            transcribe_on_miss: true,
        };
        let encoded = payload.encode();
        assert_eq!(SubtitleJobPayload::decode(&encoded).unwrap(), payload);
    }

    #[test]
    fn decode_defaults_missing_transcribe_on_miss_to_false() {
        let legacy = r#"{"version_id":"v1","imdb_id":null,"title":null,"languages":[],"season":null,"episode":null}"#;
        assert!(!SubtitleJobPayload::decode(legacy).unwrap().transcribe_on_miss);
    }

    #[test]
    fn decode_rejects_malformed_json() {
        assert!(SubtitleJobPayload::decode("not json").is_err());
    }
}

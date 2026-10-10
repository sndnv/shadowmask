use domain::catalog::VersionId;
use serde::{Deserialize, Serialize};

use crate::job::encode_payload;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackPrepJobPayload {
    pub version_id: VersionId,
}

impl PlaybackPrepJobPayload {
    pub fn encode(&self) -> String {
        encode_payload(&Wire { version_id: self.version_id.0.clone() })
    }

    pub fn decode(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str::<Wire>(raw)
            .map(|wire| PlaybackPrepJobPayload { version_id: VersionId(wire.version_id) })
    }
}

#[derive(Serialize, Deserialize)]
struct Wire {
    version_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let payload = PlaybackPrepJobPayload { version_id: VersionId("v1".into()) };
        assert_eq!(PlaybackPrepJobPayload::decode(&payload.encode()).unwrap(), payload);
    }

    #[test]
    fn decode_rejects_malformed_json() {
        assert!(PlaybackPrepJobPayload::decode("not json").is_err());
    }
}

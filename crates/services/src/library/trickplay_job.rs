use domain::catalog::VersionId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrickplayJobPayload {
    pub version_id: VersionId,
    pub source_path: String,
    pub duration_ms: u64,
}

impl TrickplayJobPayload {
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
    source_path: String,
    duration_ms: u64,
}

impl From<&TrickplayJobPayload> for Wire {
    fn from(payload: &TrickplayJobPayload) -> Self {
        Wire {
            version_id: payload.version_id.0.clone(),
            source_path: payload.source_path.clone(),
            duration_ms: payload.duration_ms,
        }
    }
}

impl From<Wire> for TrickplayJobPayload {
    fn from(wire: Wire) -> Self {
        TrickplayJobPayload {
            version_id: VersionId(wire.version_id),
            source_path: wire.source_path,
            duration_ms: wire.duration_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let payload = TrickplayJobPayload {
            version_id: VersionId("v1".into()),
            source_path: "/media/v1.mkv".into(),
            duration_ms: 1_200_000,
        };
        let encoded = payload.encode().unwrap();
        assert_eq!(TrickplayJobPayload::decode(&encoded).unwrap(), payload);
    }

    #[test]
    fn decode_rejects_malformed_json() {
        assert!(TrickplayJobPayload::decode("not json").is_err());
    }
}

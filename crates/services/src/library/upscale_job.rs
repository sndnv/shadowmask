use domain::catalog::VersionId;
use domain::job::{Job, JobKind, JobPriority};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::job::{encode_payload, queued_job};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpscaleJobPayload {
    pub version_id: VersionId,
    pub target_height: u32,
}

impl UpscaleJobPayload {
    pub fn encode(&self) -> String {
        encode_payload(&Wire::from(self))
    }

    pub fn decode(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str::<Wire>(raw).map(Self::from)
    }
}

pub fn upscale_job(version_id: &VersionId, target_height: u32) -> Job {
    let raw = UpscaleJobPayload {
        version_id: version_id.clone(),
        target_height,
    }
    .encode();
    queued_job(
        JobKind::Upscale,
        JobPriority::Low,
        raw,
        None,
        Timestamp::now(),
    )
}

#[derive(Serialize, Deserialize)]
struct Wire {
    version_id: String,
    target_height: u32,
}

impl From<&UpscaleJobPayload> for Wire {
    fn from(payload: &UpscaleJobPayload) -> Self {
        Wire {
            version_id: payload.version_id.0.clone(),
            target_height: payload.target_height,
        }
    }
}

impl From<Wire> for UpscaleJobPayload {
    fn from(wire: Wire) -> Self {
        UpscaleJobPayload {
            version_id: VersionId(wire.version_id),
            target_height: wire.target_height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let payload = UpscaleJobPayload {
            version_id: VersionId("v1".into()),
            target_height: 1080,
        };
        let encoded = payload.encode();
        assert_eq!(UpscaleJobPayload::decode(&encoded).unwrap(), payload);
    }

    #[test]
    fn decode_rejects_malformed_json() {
        assert!(UpscaleJobPayload::decode("not json").is_err());
    }

    #[test]
    fn builder_produces_a_low_priority_upscale_job() {
        let job = upscale_job(&VersionId("v9".into()), 2160);
        assert_eq!(job.kind, JobKind::Upscale);
        assert_eq!(job.priority, JobPriority::Low);
        let decoded = UpscaleJobPayload::decode(&job.payload).unwrap();
        assert_eq!(decoded.version_id, VersionId("v9".into()));
        assert_eq!(decoded.target_height, 2160);
    }
}

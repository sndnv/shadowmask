use domain::catalog::VersionId;
use domain::job::{Job, JobKind, JobPriority};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::job::{encode_payload, queued_job};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombineJobPayload {
    pub version_id: VersionId,
    pub top_subtitle_id: String,
    pub bottom_subtitle_id: String,
}

impl CombineJobPayload {
    pub fn encode(&self) -> String {
        encode_payload(&Wire::from(self))
    }

    pub fn decode(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str::<Wire>(raw).map(Self::from)
    }
}

pub fn combine_job(version_id: &VersionId, top_id: &str, bottom_id: &str) -> Job {
    let raw = CombineJobPayload {
        version_id: version_id.clone(),
        top_subtitle_id: top_id.to_owned(),
        bottom_subtitle_id: bottom_id.to_owned(),
    }
    .encode();
    queued_job(JobKind::Combine, JobPriority::Low, raw, None, Timestamp::now())
}

#[derive(Serialize, Deserialize)]
struct Wire {
    version_id: String,
    top_subtitle_id: String,
    bottom_subtitle_id: String,
}

impl From<&CombineJobPayload> for Wire {
    fn from(payload: &CombineJobPayload) -> Self {
        Wire {
            version_id: payload.version_id.0.clone(),
            top_subtitle_id: payload.top_subtitle_id.clone(),
            bottom_subtitle_id: payload.bottom_subtitle_id.clone(),
        }
    }
}

impl From<Wire> for CombineJobPayload {
    fn from(wire: Wire) -> Self {
        CombineJobPayload {
            version_id: VersionId(wire.version_id),
            top_subtitle_id: wire.top_subtitle_id,
            bottom_subtitle_id: wire.bottom_subtitle_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let payload = CombineJobPayload {
            version_id: VersionId("v1".into()),
            top_subtitle_id: "sf-en".into(),
            bottom_subtitle_id: "sf-fr".into(),
        };
        let encoded = payload.encode();
        assert_eq!(CombineJobPayload::decode(&encoded).unwrap(), payload);
    }

    #[test]
    fn decode_rejects_malformed_json() {
        assert!(CombineJobPayload::decode("not json").is_err());
    }

    #[test]
    fn combine_job_builds_low_priority_combine_job() {
        let job = combine_job(&VersionId("v1".into()), "sf-en", "sf-fr");
        assert_eq!(job.kind, JobKind::Combine);
        assert_eq!(job.priority, JobPriority::Low);
        let decoded = CombineJobPayload::decode(&job.payload).unwrap();
        assert_eq!(decoded.version_id, VersionId("v1".into()));
        assert_eq!(decoded.top_subtitle_id, "sf-en");
        assert_eq!(decoded.bottom_subtitle_id, "sf-fr");
    }
}

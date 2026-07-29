use domain::catalog::VersionId;
use domain::job::{Job, JobId, JobKind, JobPriority, JobStatus};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombineJobPayload {
    pub version_id: VersionId,
    pub primary_subtitle_id: String,
    pub secondary_subtitle_id: String,
}

impl CombineJobPayload {
    pub fn encode(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&Wire::from(self))
    }

    pub fn decode(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str::<Wire>(raw).map(Self::from)
    }
}

pub fn combine_job(version_id: &VersionId, primary_id: &str, secondary_id: &str) -> Job {
    let raw = CombineJobPayload {
        version_id: version_id.clone(),
        primary_subtitle_id: primary_id.to_owned(),
        secondary_subtitle_id: secondary_id.to_owned(),
    }
    .encode()
    .expect("combine job payload serializes");
    let now = Timestamp::now();
    Job {
        id: JobId(Uuid::new_v4().to_string()),
        kind: JobKind::Combine,
        status: JobStatus::Queued,
        priority: JobPriority::Low,
        payload: raw,
        attempts: 0,
        progress: 0.0,
        available_at: now,
        last_error: None,
        created_at: now,
        updated_at: now,
        started_at: None,
        finished_at: None,
        parent_id: None,
    }
}

#[derive(Serialize, Deserialize)]
struct Wire {
    version_id: String,
    primary_subtitle_id: String,
    secondary_subtitle_id: String,
}

impl From<&CombineJobPayload> for Wire {
    fn from(payload: &CombineJobPayload) -> Self {
        Wire {
            version_id: payload.version_id.0.clone(),
            primary_subtitle_id: payload.primary_subtitle_id.clone(),
            secondary_subtitle_id: payload.secondary_subtitle_id.clone(),
        }
    }
}

impl From<Wire> for CombineJobPayload {
    fn from(wire: Wire) -> Self {
        CombineJobPayload {
            version_id: VersionId(wire.version_id),
            primary_subtitle_id: wire.primary_subtitle_id,
            secondary_subtitle_id: wire.secondary_subtitle_id,
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
            primary_subtitle_id: "sf-en".into(),
            secondary_subtitle_id: "sf-fr".into(),
        };
        let encoded = payload.encode().unwrap();
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
        assert_eq!(decoded.primary_subtitle_id, "sf-en");
        assert_eq!(decoded.secondary_subtitle_id, "sf-fr");
    }
}

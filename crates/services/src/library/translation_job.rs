use domain::catalog::VersionId;
use domain::job::{Job, JobKind, JobPriority};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::job::{encode_payload, queued_job};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationJobPayload {
    pub version_id: VersionId,
    pub target_languages: Vec<String>,
    pub source_subtitle_id: Option<String>,
}

impl TranslationJobPayload {
    pub fn encode(&self) -> String {
        encode_payload(&Wire::from(self))
    }

    pub fn decode(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str::<Wire>(raw).map(Self::from)
    }
}

pub fn translation_job(version_id: &VersionId, target_languages: &[String]) -> Option<Job> {
    if target_languages.is_empty() {
        return None;
    }
    let raw = TranslationJobPayload {
        version_id: version_id.clone(),
        target_languages: target_languages.to_vec(),
        source_subtitle_id: None,
    }
    .encode();
    Some(translation_job_from_payload(raw))
}

pub fn translation_job_with_source(
    version_id: &VersionId,
    source_subtitle_id: &str,
    target_language: &str,
) -> Job {
    let raw = TranslationJobPayload {
        version_id: version_id.clone(),
        target_languages: vec![target_language.to_owned()],
        source_subtitle_id: Some(source_subtitle_id.to_owned()),
    }
    .encode();
    translation_job_from_payload(raw)
}

fn translation_job_from_payload(payload: String) -> Job {
    queued_job(JobKind::Translation, JobPriority::Low, payload, None, Timestamp::now())
}

#[derive(Serialize, Deserialize)]
struct Wire {
    version_id: String,
    target_languages: Vec<String>,
    #[serde(default)]
    source_subtitle_id: Option<String>,
}

impl From<&TranslationJobPayload> for Wire {
    fn from(payload: &TranslationJobPayload) -> Self {
        Wire {
            version_id: payload.version_id.0.clone(),
            target_languages: payload.target_languages.clone(),
            source_subtitle_id: payload.source_subtitle_id.clone(),
        }
    }
}

impl From<Wire> for TranslationJobPayload {
    fn from(wire: Wire) -> Self {
        TranslationJobPayload {
            version_id: VersionId(wire.version_id),
            target_languages: wire.target_languages,
            source_subtitle_id: wire.source_subtitle_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let payload = TranslationJobPayload {
            version_id: VersionId("v1".into()),
            target_languages: vec!["fr".into(), "de".into()],
            source_subtitle_id: Some("sf1".into()),
        };
        let encoded = payload.encode();
        assert_eq!(TranslationJobPayload::decode(&encoded).unwrap(), payload);
    }

    #[test]
    fn decode_defaults_missing_source_subtitle_id_to_none() {
        let legacy = r#"{"version_id":"v1","target_languages":["fr"]}"#;
        let decoded = TranslationJobPayload::decode(legacy).unwrap();
        assert_eq!(decoded.source_subtitle_id, None);
        assert_eq!(decoded.target_languages, vec!["fr".to_owned()]);
    }

    #[test]
    fn decode_rejects_malformed_json() {
        assert!(TranslationJobPayload::decode("not json").is_err());
    }

    #[test]
    fn translation_job_with_source_targets_a_single_language_from_a_chosen_file() {
        let job = translation_job_with_source(&VersionId("v1".into()), "sf1", "zh");
        assert_eq!(job.kind, JobKind::Translation);
        assert_eq!(job.priority, JobPriority::Low);
        let decoded = TranslationJobPayload::decode(&job.payload).unwrap();
        assert_eq!(decoded.target_languages, vec!["zh".to_owned()]);
        assert_eq!(decoded.source_subtitle_id.as_deref(), Some("sf1"));
    }

    #[test]
    fn translation_job_is_none_without_languages() {
        assert!(translation_job(&VersionId("v1".into()), &[]).is_none());
    }

    #[test]
    fn translation_job_builds_low_priority_translation_job() {
        let job = translation_job(&VersionId("v1".into()), &["fr".into()]).unwrap();
        assert_eq!(job.kind, JobKind::Translation);
        assert_eq!(job.priority, JobPriority::Low);
        let decoded = TranslationJobPayload::decode(&job.payload).unwrap();
        assert_eq!(decoded.version_id, VersionId("v1".into()));
        assert_eq!(decoded.target_languages, vec!["fr".to_owned()]);
    }
}

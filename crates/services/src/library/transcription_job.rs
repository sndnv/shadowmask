use domain::catalog::VersionId;
use domain::job::{Job, JobId, JobKind, JobPriority, JobStatus};
use domain::media::AudioTrack;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptionJobPayload {
    pub version_id: VersionId,
    pub source_path: String,
    pub source_language: Option<String>,
    pub audio_track_index: Option<u32>,
}

impl TranscriptionJobPayload {
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
    source_language: Option<String>,
    #[serde(default)]
    audio_track_index: Option<u32>,
}

impl From<&TranscriptionJobPayload> for Wire {
    fn from(payload: &TranscriptionJobPayload) -> Self {
        Wire {
            version_id: payload.version_id.0.clone(),
            source_path: payload.source_path.clone(),
            source_language: payload.source_language.clone(),
            audio_track_index: payload.audio_track_index,
        }
    }
}

impl From<Wire> for TranscriptionJobPayload {
    fn from(wire: Wire) -> Self {
        TranscriptionJobPayload {
            version_id: VersionId(wire.version_id),
            source_path: wire.source_path,
            source_language: wire.source_language,
            audio_track_index: wire.audio_track_index,
        }
    }
}

pub fn transcription_job(
    version_id: &VersionId,
    source_path: &str,
    source_language: Option<String>,
    audio_track_index: Option<u32>,
) -> Job {
    let raw = TranscriptionJobPayload {
        version_id: version_id.clone(),
        source_path: source_path.to_owned(),
        source_language,
        audio_track_index,
    }
    .encode()
    .expect("transcription job payload serializes");
    let now = Timestamp::now();
    Job {
        id: JobId(Uuid::new_v4().to_string()),
        kind: JobKind::Transcription,
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

pub fn select_audio_track(tracks: &[AudioTrack], preferred: Option<&str>) -> Option<u32> {
    let preferred = preferred?;
    tracks
        .iter()
        .find(|track| track.language.as_ref().map(|l| l.0.as_str()) == Some(preferred))
        .map(|track| track.index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let payload = TranscriptionJobPayload {
            version_id: VersionId("v1".into()),
            source_path: "/media/v1.mkv".into(),
            source_language: Some("en".into()),
            audio_track_index: Some(2),
        };
        let encoded = payload.encode().unwrap();
        assert_eq!(TranscriptionJobPayload::decode(&encoded).unwrap(), payload);
    }

    #[test]
    fn round_trips_without_language() {
        let payload = TranscriptionJobPayload {
            version_id: VersionId("v2".into()),
            source_path: "/media/v2.mkv".into(),
            source_language: None,
            audio_track_index: None,
        };
        let encoded = payload.encode().unwrap();
        assert_eq!(TranscriptionJobPayload::decode(&encoded).unwrap(), payload);
    }

    #[test]
    fn decode_defaults_missing_audio_track_index_to_none() {
        let legacy = r#"{"version_id":"v3","source_path":"/media/v3.mkv","source_language":null}"#;
        let decoded = TranscriptionJobPayload::decode(legacy).unwrap();
        assert_eq!(decoded.audio_track_index, None);
        assert_eq!(decoded.version_id, VersionId("v3".into()));
    }

    #[test]
    fn decode_rejects_malformed_json() {
        assert!(TranscriptionJobPayload::decode("not json").is_err());
    }

    #[test]
    fn transcription_job_builds_low_priority_job_with_selection() {
        let job = transcription_job(
            &VersionId("v1".into()),
            "/media/v1.mkv",
            Some("spa".into()),
            Some(2),
        );
        assert_eq!(job.kind, JobKind::Transcription);
        assert_eq!(job.priority, JobPriority::Low);
        let decoded = TranscriptionJobPayload::decode(&job.payload).unwrap();
        assert_eq!(decoded.version_id, VersionId("v1".into()));
        assert_eq!(decoded.source_path, "/media/v1.mkv");
        assert_eq!(decoded.source_language.as_deref(), Some("spa"));
        assert_eq!(decoded.audio_track_index, Some(2));
    }

    fn track(index: u32, language: Option<&str>) -> AudioTrack {
        AudioTrack {
            index,
            codec: "aac".into(),
            channels: 2,
            language: language.map(|l| domain::common::LanguageCode(l.to_owned())),
            bitrate: None,
        }
    }

    #[test]
    fn select_audio_track_returns_index_of_preferred_language() {
        let tracks = [track(1, Some("eng")), track(2, Some("spa"))];
        assert_eq!(select_audio_track(&tracks, Some("spa")), Some(2));
    }

    #[test]
    fn select_audio_track_returns_first_match() {
        let tracks = [track(1, Some("eng")), track(2, Some("eng"))];
        assert_eq!(select_audio_track(&tracks, Some("eng")), Some(1));
    }

    #[test]
    fn select_audio_track_is_none_without_a_match() {
        let tracks = [track(1, Some("eng")), track(2, None)];
        assert_eq!(select_audio_track(&tracks, Some("spa")), None);
    }

    #[test]
    fn select_audio_track_is_none_without_a_preference() {
        let tracks = [track(1, Some("eng"))];
        assert_eq!(select_audio_track(&tracks, None), None);
    }

    #[test]
    fn select_audio_track_is_none_when_empty() {
        assert_eq!(select_audio_track(&[], Some("eng")), None);
    }
}

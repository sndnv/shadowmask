use serde::Serialize;

use domain::job::{Job, JobKind, JobPriority, JobStatus};

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKindDto {
    LibraryScan,
    Metadata,
    Artwork,
    Subtitles,
    Trickplay,
    Fingerprint,
    Dedup,
    CacheEviction,
    SearchReindex,
    Ingest,
    Relink,
    Transcription,
    Translation,
    Upscale,
    Combine,
    Fetch,
    ScheduledScan,
    Retention,
    OrphanSweep,
}

impl From<JobKind> for JobKindDto {
    fn from(k: JobKind) -> Self {
        match k {
            JobKind::LibraryScan => JobKindDto::LibraryScan,
            JobKind::Metadata => JobKindDto::Metadata,
            JobKind::Artwork => JobKindDto::Artwork,
            JobKind::Subtitles => JobKindDto::Subtitles,
            JobKind::Trickplay => JobKindDto::Trickplay,
            JobKind::Fingerprint => JobKindDto::Fingerprint,
            JobKind::Dedup => JobKindDto::Dedup,
            JobKind::CacheEviction => JobKindDto::CacheEviction,
            JobKind::SearchReindex => JobKindDto::SearchReindex,
            JobKind::Ingest => JobKindDto::Ingest,
            JobKind::Relink => JobKindDto::Relink,
            JobKind::Transcription => JobKindDto::Transcription,
            JobKind::Translation => JobKindDto::Translation,
            JobKind::Upscale => JobKindDto::Upscale,
            JobKind::Combine => JobKindDto::Combine,
            JobKind::Fetch => JobKindDto::Fetch,
            JobKind::ScheduledScan => JobKindDto::ScheduledScan,
            JobKind::Retention => JobKindDto::Retention,
            JobKind::OrphanSweep => JobKindDto::OrphanSweep,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatusDto {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl From<JobStatus> for JobStatusDto {
    fn from(s: JobStatus) -> Self {
        match s {
            JobStatus::Queued => JobStatusDto::Queued,
            JobStatus::Running => JobStatusDto::Running,
            JobStatus::Succeeded => JobStatusDto::Succeeded,
            JobStatus::Failed => JobStatusDto::Failed,
            JobStatus::Cancelled => JobStatusDto::Cancelled,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobPriorityDto {
    Low,
    Normal,
    High,
}

impl From<JobPriority> for JobPriorityDto {
    fn from(p: JobPriority) -> Self {
        match p {
            JobPriority::Low => JobPriorityDto::Low,
            JobPriority::Normal => JobPriorityDto::Normal,
            JobPriority::High => JobPriorityDto::High,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct JobLogResponse {
    pub lines: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct JobResponse {
    pub id: String,
    pub kind: JobKindDto,
    pub status: JobStatusDto,
    pub priority: JobPriorityDto,
    pub progress: f32,
    pub attempts: u32,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub parent_id: Option<String>,
    pub cancellable: bool,
}

impl From<Job> for JobResponse {
    fn from(j: Job) -> Self {
        let cancellable = match j.status {
            JobStatus::Queued => true,
            JobStatus::Running => j.kind.is_process_killable(),
            _ => false,
        };
        JobResponse {
            id: j.id.0,
            kind: j.kind.into(),
            status: j.status.into(),
            priority: j.priority.into(),
            progress: j.progress,
            attempts: j.attempts,
            last_error: j.last_error,
            created_at: j.created_at.to_string(),
            updated_at: j.updated_at.to_string(),
            started_at: j.started_at.map(|t| t.to_string()),
            finished_at: j.finished_at.map(|t| t.to_string()),
            parent_id: j.parent_id.map(|id| id.0),
            cancellable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_all_kinds() {
        for (kind, expected) in [
            (JobKind::LibraryScan, "library_scan"),
            (JobKind::Metadata, "metadata"),
            (JobKind::Artwork, "artwork"),
            (JobKind::Subtitles, "subtitles"),
            (JobKind::Trickplay, "trickplay"),
            (JobKind::Fingerprint, "fingerprint"),
            (JobKind::Dedup, "dedup"),
            (JobKind::CacheEviction, "cache_eviction"),
            (JobKind::SearchReindex, "search_reindex"),
            (JobKind::Ingest, "ingest"),
            (JobKind::Relink, "relink"),
            (JobKind::Transcription, "transcription"),
            (JobKind::Translation, "translation"),
            (JobKind::Upscale, "upscale"),
            (JobKind::Combine, "combine"),
            (JobKind::Fetch, "fetch"),
            (JobKind::ScheduledScan, "scheduled_scan"),
            (JobKind::Retention, "retention"),
            (JobKind::OrphanSweep, "orphan_sweep"),
        ] {
            let dto = JobKindDto::from(kind);
            assert_eq!(serde_json::to_value(dto).unwrap(), expected);
        }
    }

    #[test]
    fn maps_parent_id() {
        use domain::job::JobId;
        use jiff::Timestamp;

        let job = Job {
            id: JobId("child".into()),
            kind: JobKind::Translation,
            status: JobStatus::Queued,
            priority: JobPriority::Low,
            payload: String::new(),
            attempts: 0,
            progress: 0.0,
            available_at: Timestamp::UNIX_EPOCH,
            last_error: None,
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            started_at: None,
            finished_at: None,
            parent_id: Some(JobId("parent".into())),
        };
        assert_eq!(JobResponse::from(job).parent_id.as_deref(), Some("parent"));
    }

    #[test]
    fn computes_cancellable_from_status_and_kind() {
        use domain::job::JobId;
        use jiff::Timestamp;

        let build = |kind, status| Job {
            id: JobId("j".into()),
            kind,
            status,
            priority: JobPriority::Normal,
            payload: String::new(),
            attempts: 0,
            progress: 0.0,
            available_at: Timestamp::UNIX_EPOCH,
            last_error: None,
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            started_at: None,
            finished_at: None,
            parent_id: None,
        };
        assert!(JobResponse::from(build(JobKind::LibraryScan, JobStatus::Queued)).cancellable);
        assert!(JobResponse::from(build(JobKind::Fetch, JobStatus::Running)).cancellable);
        assert!(!JobResponse::from(build(JobKind::LibraryScan, JobStatus::Running)).cancellable);
        assert!(!JobResponse::from(build(JobKind::Fetch, JobStatus::Succeeded)).cancellable);
    }

    #[test]
    fn maps_all_statuses_and_priorities() {
        for (status, expected) in [
            (JobStatus::Queued, "queued"),
            (JobStatus::Running, "running"),
            (JobStatus::Succeeded, "succeeded"),
            (JobStatus::Failed, "failed"),
            (JobStatus::Cancelled, "cancelled"),
        ] {
            let dto = JobStatusDto::from(status);
            assert_eq!(serde_json::to_value(dto).unwrap(), expected);
        }
        for (priority, expected) in [
            (JobPriority::Low, "low"),
            (JobPriority::Normal, "normal"),
            (JobPriority::High, "high"),
        ] {
            let dto = JobPriorityDto::from(priority);
            assert_eq!(serde_json::to_value(dto).unwrap(), expected);
        }
    }
}

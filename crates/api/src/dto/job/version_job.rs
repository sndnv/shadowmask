use serde::Serialize;

use domain::job::VersionJob;

use super::job::{JobKindDto, JobStatusDto};

#[derive(Debug, Serialize)]
pub struct VersionJobResponse {
    pub id: String,
    pub kind: JobKindDto,
    pub status: JobStatusDto,
    pub ahead: Option<u64>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub elapsed_ms: Option<u64>,
    pub last_error: Option<String>,
    pub language: Option<String>,
    pub subtitle_id: Option<String>,
}

impl From<VersionJob> for VersionJobResponse {
    fn from(entry: VersionJob) -> Self {
        let job = entry.job;
        VersionJobResponse {
            id: job.id.0,
            kind: job.kind.into(),
            status: job.status.into(),
            ahead: entry.ahead,
            created_at: job.created_at.to_string(),
            started_at: job.started_at.map(|t| t.to_string()),
            finished_at: job.finished_at.map(|t| t.to_string()),
            elapsed_ms: entry.elapsed_ms,
            last_error: job.last_error,
            language: entry.language,
            subtitle_id: entry.subtitle.map(|id| id.0),
        }
    }
}

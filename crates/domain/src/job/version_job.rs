use crate::job::Job;
use crate::media::SubtitleFileId;

#[derive(Debug, Clone)]
pub struct VersionJob {
    pub job: Job,
    pub ahead: Option<u64>,
    pub elapsed_ms: Option<u64>,
    pub language: Option<String>,
    pub subtitle: Option<SubtitleFileId>,
}

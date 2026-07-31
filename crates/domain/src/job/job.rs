use jiff::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobKind {
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
}

impl JobKind {
    pub fn is_process_killable(&self) -> bool {
        matches!(
            self,
            JobKind::Trickplay | JobKind::Upscale | JobKind::Subtitles | JobKind::Fetch
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JobPriority {
    Low,
    Normal,
    High,
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: JobId,
    pub kind: JobKind,
    pub status: JobStatus,
    pub priority: JobPriority,
    pub payload: String,
    pub attempts: u32,
    pub progress: f32,
    pub available_at: Timestamp,
    pub last_error: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub started_at: Option<Timestamp>,
    pub finished_at: Option<Timestamp>,
    pub parent_id: Option<JobId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_killable_kinds() {
        for kind in [
            JobKind::Trickplay,
            JobKind::Upscale,
            JobKind::Subtitles,
            JobKind::Fetch,
        ] {
            assert!(kind.is_process_killable(), "{kind:?} should be killable");
        }
        for kind in [
            JobKind::LibraryScan,
            JobKind::Metadata,
            JobKind::Artwork,
            JobKind::Fingerprint,
            JobKind::Dedup,
            JobKind::CacheEviction,
            JobKind::SearchReindex,
            JobKind::Ingest,
            JobKind::Relink,
            JobKind::Transcription,
            JobKind::Translation,
            JobKind::Combine,
        ] {
            assert!(
                !kind.is_process_killable(),
                "{kind:?} should not be killable"
            );
        }
    }
}

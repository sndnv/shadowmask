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

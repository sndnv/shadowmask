use jiff::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

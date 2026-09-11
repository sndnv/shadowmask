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
    ScheduledScan,
    Retention,
    OrphanSweep,
}

impl JobKind {
    pub fn is_process_killable(&self) -> bool {
        matches!(self, JobKind::Trickplay | JobKind::Upscale | JobKind::Subtitles | JobKind::Fetch)
    }

    pub fn slug(&self) -> &'static str {
        match self {
            JobKind::LibraryScan => "library_scan",
            JobKind::Metadata => "metadata",
            JobKind::Artwork => "artwork",
            JobKind::Subtitles => "subtitles",
            JobKind::Trickplay => "trickplay",
            JobKind::Fingerprint => "fingerprint",
            JobKind::Dedup => "dedup",
            JobKind::CacheEviction => "cache_eviction",
            JobKind::SearchReindex => "search_reindex",
            JobKind::Ingest => "ingest",
            JobKind::Relink => "relink",
            JobKind::Transcription => "transcription",
            JobKind::Translation => "translation",
            JobKind::Upscale => "upscale",
            JobKind::Combine => "combine",
            JobKind::Fetch => "fetch",
            JobKind::ScheduledScan => "scheduled_scan",
            JobKind::Retention => "retention",
            JobKind::OrphanSweep => "orphan_sweep",
        }
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

impl JobStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, JobStatus::Queued | JobStatus::Running)
    }

    pub fn slug(&self) -> &'static str {
        match self {
            JobStatus::Queued => "queued",
            JobStatus::Running => "running",
            JobStatus::Succeeded => "succeeded",
            JobStatus::Failed => "failed",
            JobStatus::Cancelled => "cancelled",
        }
    }
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

impl Job {
    pub fn queued(
        id: JobId,
        kind: JobKind,
        priority: JobPriority,
        payload: String,
        parent: Option<JobId>,
        now: Timestamp,
    ) -> Self {
        Self {
            id,
            kind,
            status: JobStatus::Queued,
            priority,
            payload,
            attempts: 0,
            progress: 0.0,
            available_at: now,
            last_error: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            finished_at: None,
            parent_id: parent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_a_distinct_slug() {
        let kinds = [
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
        ];
        for (kind, slug) in kinds {
            assert_eq!(kind.slug(), slug);
        }
        let unique: std::collections::HashSet<&str> = kinds.iter().map(|(_, slug)| *slug).collect();
        assert_eq!(unique.len(), kinds.len());
    }

    #[test]
    fn every_status_has_a_slug_and_knows_if_it_is_active() {
        for (status, slug, active) in [
            (JobStatus::Queued, "queued", true),
            (JobStatus::Running, "running", true),
            (JobStatus::Succeeded, "succeeded", false),
            (JobStatus::Failed, "failed", false),
            (JobStatus::Cancelled, "cancelled", false),
        ] {
            assert_eq!(status.slug(), slug);
            assert_eq!(status.is_active(), active, "{slug}");
        }
    }

    #[test]
    fn process_killable_kinds() {
        for kind in [JobKind::Trickplay, JobKind::Upscale, JobKind::Subtitles, JobKind::Fetch] {
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
            JobKind::ScheduledScan,
        ] {
            assert!(!kind.is_process_killable(), "{kind:?} should not be killable");
        }
    }
}

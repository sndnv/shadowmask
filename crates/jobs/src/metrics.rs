use std::time::Instant;

use domain::job::JobKind;

pub(crate) struct ActiveJob {
    job: &'static str,
    start: Instant,
}

impl ActiveJob {
    pub(crate) fn start(kind: JobKind) -> Self {
        metrics::gauge!("jobs_active").increment(1.0);
        Self {
            job: job_label(kind),
            start: Instant::now(),
        }
    }

    pub(crate) fn finish(self, succeeded: bool) {
        let outcome = if succeeded { "succeeded" } else { "failed" };
        metrics::histogram!("job_duration_milliseconds", "job" => self.job)
            .record(self.start.elapsed().as_nanos() as f64 / 1_000_000.0);
        metrics::counter!("jobs_total", "job" => self.job, "outcome" => outcome).increment(1);
    }
}

impl Drop for ActiveJob {
    fn drop(&mut self) {
        metrics::gauge!("jobs_active").decrement(1.0);
    }
}

fn job_label(kind: JobKind) -> &'static str {
    match kind {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_label_covers_all_kinds() {
        for kind in [
            JobKind::LibraryScan,
            JobKind::Metadata,
            JobKind::Artwork,
            JobKind::Subtitles,
            JobKind::Trickplay,
            JobKind::Fingerprint,
            JobKind::Dedup,
            JobKind::CacheEviction,
            JobKind::SearchReindex,
            JobKind::Ingest,
        ] {
            assert!(!job_label(kind).is_empty());
        }
    }
}

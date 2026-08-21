use crate::job::Job;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JobQuery {
    pub search: Option<String>,
    pub active_only: bool,
}

impl JobQuery {
    pub fn active(active_only: bool) -> Self {
        JobQuery {
            search: None,
            active_only,
        }
    }

    pub fn needle(&self) -> Option<String> {
        let needle = self
            .search
            .as_deref()?
            .trim()
            .to_lowercase()
            .replace('_', " ");
        (!needle.is_empty()).then_some(needle)
    }

    pub fn matches(&self, job: &Job) -> bool {
        if self.active_only && !job.status.is_active() {
            return false;
        }
        match self.needle() {
            None => true,
            Some(needle) => haystack(job).contains(&needle),
        }
    }
}

fn haystack(job: &Job) -> String {
    format!(
        "{} {} {}",
        job.id.0.to_lowercase(),
        job.kind.slug().replace('_', " "),
        job.status.slug()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::{JobId, JobKind, JobPriority, JobStatus};
    use jiff::Timestamp;

    fn job(id: &str, kind: JobKind, status: JobStatus) -> Job {
        Job {
            id: JobId(id.into()),
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
        }
    }

    #[test]
    fn an_empty_query_matches_everything() {
        let query = JobQuery::default();
        assert!(query.matches(&job("a", JobKind::Artwork, JobStatus::Failed)));
        assert!(query.needle().is_none());
    }

    #[test]
    fn blank_search_text_is_not_a_filter() {
        let query = JobQuery {
            search: Some("   ".into()),
            active_only: false,
        };
        assert!(query.needle().is_none());
        assert!(query.matches(&job("a", JobKind::Artwork, JobStatus::Failed)));
    }

    #[test]
    fn search_covers_id_kind_and_status() {
        let target = job("job-7f", JobKind::LibraryScan, JobStatus::Running);
        for text in ["JOB-7F", "library scan", "library_scan", "running"] {
            let query = JobQuery {
                search: Some(text.into()),
                active_only: false,
            };
            assert!(query.matches(&target), "{text} should match");
        }
        let query = JobQuery {
            search: Some("artwork".into()),
            active_only: false,
        };
        assert!(!query.matches(&target));
    }

    #[test]
    fn active_only_keeps_queued_and_running() {
        let query = JobQuery::active(true);
        assert!(query.matches(&job("a", JobKind::Artwork, JobStatus::Queued)));
        assert!(query.matches(&job("b", JobKind::Artwork, JobStatus::Running)));
        assert!(!query.matches(&job("c", JobKind::Artwork, JobStatus::Succeeded)));
        assert!(!query.matches(&job("d", JobKind::Artwork, JobStatus::Failed)));
        assert!(!query.matches(&job("e", JobKind::Artwork, JobStatus::Cancelled)));
    }

    #[test]
    fn active_and_search_both_apply() {
        let query = JobQuery {
            search: Some("artwork".into()),
            active_only: true,
        };
        assert!(query.matches(&job("a", JobKind::Artwork, JobStatus::Queued)));
        assert!(!query.matches(&job("b", JobKind::Artwork, JobStatus::Succeeded)));
        assert!(!query.matches(&job("c", JobKind::Metadata, JobStatus::Queued)));
    }
}

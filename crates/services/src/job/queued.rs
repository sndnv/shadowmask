use domain::job::{Job, JobId, JobKind, JobPriority};
use jiff::Timestamp;
use uuid::Uuid;

pub fn queued_job(
    kind: JobKind,
    priority: JobPriority,
    payload: String,
    parent: Option<&JobId>,
    now: Timestamp,
) -> Job {
    Job::queued(
        JobId(Uuid::new_v4().to_string()),
        kind,
        priority,
        payload,
        parent.cloned(),
        now,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::job::JobStatus;

    #[test]
    fn a_queued_job_starts_unattempted_and_available_now() {
        let now = Timestamp::UNIX_EPOCH;

        let job = queued_job(
            JobKind::Metadata,
            JobPriority::Normal,
            "payload".into(),
            None,
            now,
        );

        assert_eq!(job.status, JobStatus::Queued);
        assert_eq!(job.attempts, 0);
        assert_eq!(job.progress, 0.0);
        assert_eq!(job.available_at, now);
        assert_eq!(job.created_at, now);
        assert_eq!(job.updated_at, now);
        assert!(job.last_error.is_none());
        assert!(job.started_at.is_none());
        assert!(job.finished_at.is_none());
        assert!(job.parent_id.is_none());
    }

    #[test]
    fn each_queued_job_gets_its_own_id() {
        let now = Timestamp::UNIX_EPOCH;

        let first = queued_job(JobKind::Artwork, JobPriority::Low, "a".into(), None, now);
        let second = queued_job(JobKind::Artwork, JobPriority::Low, "a".into(), None, now);

        assert_ne!(first.id, second.id);
    }

    #[test]
    fn a_parent_is_carried_onto_the_child() {
        let parent = JobId("parent".into());

        let job = queued_job(
            JobKind::Subtitles,
            JobPriority::High,
            "payload".into(),
            Some(&parent),
            Timestamp::UNIX_EPOCH,
        );

        assert_eq!(job.parent_id, Some(parent));
    }
}

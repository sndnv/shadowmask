use serde::Serialize;

use domain::job::{Job, JobNode, JobPage};

use crate::dto::job::JobResponse;

#[derive(Debug, Serialize)]
pub struct JobsResponse {
    pub items: Vec<JobResponse>,
    pub total: u64,
    pub offset: u32,
    pub limit: u32,
    pub active_total: u64,
    pub all_total: u64,
}

impl JobsResponse {
    pub fn new(p: JobPage, retryable: impl Fn(&Job) -> bool) -> Self {
        JobsResponse {
            items: p
                .page
                .items
                .into_iter()
                .map(|job| {
                    let retryable = retryable(&job);
                    JobResponse::new(job, retryable)
                })
                .collect(),
            total: p.page.total,
            offset: p.page.offset,
            limit: p.page.limit,
            active_total: p.active_total,
            all_total: p.all_total,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct JobNodeResponse {
    #[serde(flatten)]
    pub job: JobResponse,
    pub depth: u32,
}

impl JobNodeResponse {
    pub fn new(n: JobNode, retryable: bool) -> Self {
        JobNodeResponse { job: JobResponse::new(n.job, retryable), depth: n.depth }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::common::Page;
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use jiff::Timestamp;

    fn job(id: &str) -> Job {
        Job {
            id: JobId(id.into()),
            kind: JobKind::Artwork,
            status: JobStatus::Queued,
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
    fn carries_both_tab_counts_alongside_the_page() {
        let response = JobsResponse::new(
            JobPage {
                page: Page { items: vec![job("a"), job("b")], total: 9, offset: 50, limit: 50 },
                active_total: 2,
                all_total: 9,
            },
            |job| job.id.0 == "b",
        );
        assert_eq!(response.items.len(), 2);
        assert!(!response.items[0].retryable);
        assert!(response.items[1].retryable, "each job asks the service on its own");
        assert_eq!(response.total, 9);
        assert_eq!(response.offset, 50);
        assert_eq!(response.limit, 50);
        assert_eq!(response.active_total, 2);
        assert_eq!(response.all_total, 9);
    }

    #[test]
    fn a_node_flattens_the_job_and_adds_its_depth() {
        let value = serde_json::to_value(JobNodeResponse::new(
            JobNode { job: job("child"), depth: 2 },
            true,
        ))
        .unwrap();
        assert_eq!(value["id"], "child");
        assert_eq!(value["depth"], 2);
        assert_eq!(value["retryable"], true);
    }
}

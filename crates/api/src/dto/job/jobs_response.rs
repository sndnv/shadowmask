use serde::Serialize;

use domain::job::{JobNode, JobPage};

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

impl From<JobPage> for JobsResponse {
    fn from(p: JobPage) -> Self {
        JobsResponse {
            items: p.page.items.into_iter().map(Into::into).collect(),
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

impl From<JobNode> for JobNodeResponse {
    fn from(n: JobNode) -> Self {
        JobNodeResponse { job: n.job.into(), depth: n.depth }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::common::Page;
    use domain::job::{Job, JobId, JobKind, JobPriority, JobStatus};
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
        let response = JobsResponse::from(JobPage {
            page: Page { items: vec![job("a")], total: 9, offset: 50, limit: 50 },
            active_total: 2,
            all_total: 9,
        });
        assert_eq!(response.items.len(), 1);
        assert_eq!(response.total, 9);
        assert_eq!(response.offset, 50);
        assert_eq!(response.limit, 50);
        assert_eq!(response.active_total, 2);
        assert_eq!(response.all_total, 9);
    }

    #[test]
    fn a_node_flattens_the_job_and_adds_its_depth() {
        let value =
            serde_json::to_value(JobNodeResponse::from(JobNode { job: job("child"), depth: 2 }))
                .unwrap();
        assert_eq!(value["id"], "child");
        assert_eq!(value["depth"], 2);
    }
}

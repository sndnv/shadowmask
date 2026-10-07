use serde::Serialize;

use domain::job::JobId;

#[derive(Debug, Serialize)]
pub struct JobStartedResponse {
    pub job_id: String,
}

impl From<JobId> for JobStartedResponse {
    fn from(id: JobId) -> Self {
        Self { job_id: id.0 }
    }
}

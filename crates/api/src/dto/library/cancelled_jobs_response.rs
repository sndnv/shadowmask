use serde::Serialize;

use domain::library::CancelledJobs;

#[derive(Debug, Serialize)]
pub struct CancelledJobsResponse {
    pub cancelled: u64,
    pub signalled: u64,
    pub running: u64,
}

impl From<CancelledJobs> for CancelledJobsResponse {
    fn from(outcome: CancelledJobs) -> Self {
        CancelledJobsResponse {
            cancelled: outcome.cancelled,
            signalled: outcome.signalled,
            running: outcome.running,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_count_is_carried_under_its_own_name() {
        let outcome = CancelledJobs { cancelled: 1, signalled: 2, running: 3 };

        assert_eq!(
            serde_json::to_value(CancelledJobsResponse::from(outcome)).unwrap(),
            serde_json::json!({"cancelled": 1, "signalled": 2, "running": 3})
        );
    }
}

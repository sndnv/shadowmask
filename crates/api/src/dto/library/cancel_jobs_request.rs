use serde::Deserialize;

use domain::library::CancelScope;

use super::CancelBlocking;

#[derive(Debug, Deserialize)]
pub struct CancelJobsRequest {
    pub blocking: Option<CancelBlocking>,
}

impl CancelJobsRequest {
    pub fn scope(&self) -> CancelScope {
        self.blocking.map_or(CancelScope::All, CancelScope::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_job_is_cancelled_unless_an_operation_is_named() {
        let purge: CancelJobsRequest =
            serde_json::from_value(serde_json::json!({"blocking": "purge"})).unwrap();
        let empty: CancelJobsRequest = serde_json::from_value(serde_json::json!({})).unwrap();

        assert_eq!(purge.scope(), CancelScope::PurgeBlockers);
        assert_eq!(empty.scope(), CancelScope::All);
    }
}

use domain::diagnostics::BenchmarkStatus;
use serde::Serialize;

use crate::dto::server::{BenchmarkProgress, BenchmarkRunResult, BenchmarkSource};

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkResponse {
    pub state: &'static str,
    pub version_id: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub source: Option<BenchmarkSource>,
    pub keyframes_ms: Option<u64>,
    pub keyframes_error: Option<String>,
    pub progress: BenchmarkProgress,
    pub runs: Vec<BenchmarkRunResult>,
}

impl From<BenchmarkStatus> for BenchmarkResponse {
    fn from(status: BenchmarkStatus) -> Self {
        let state = match (status.running, &status.target) {
            (true, _) => "running",
            (false, Some(_)) => "done",
            (false, None) => "idle",
        };
        Self {
            state,
            version_id: status.target.as_ref().map(|target| target.version.0.clone()),
            started_at: status.started_at.map(|at| at.to_string()),
            finished_at: status.finished_at.map(|at| at.to_string()),
            source: status.target.map(BenchmarkSource::from),
            keyframes_ms: status.keyframes_ms,
            keyframes_error: status.keyframes_error,
            progress: BenchmarkProgress { done: status.runs.len(), total: status.total },
            runs: status.runs.into_iter().map(BenchmarkRunResult::from).collect(),
        }
    }
}

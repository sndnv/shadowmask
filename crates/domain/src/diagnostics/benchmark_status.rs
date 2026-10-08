use jiff::Timestamp;

use crate::diagnostics::{BenchmarkRun, BenchmarkTarget};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BenchmarkStatus {
    pub running: bool,
    pub target: Option<BenchmarkTarget>,
    pub started_at: Option<Timestamp>,
    pub finished_at: Option<Timestamp>,
    pub keyframes_ms: Option<u64>,
    pub keyframes_error: Option<String>,
    pub total: usize,
    pub runs: Vec<BenchmarkRun>,
}

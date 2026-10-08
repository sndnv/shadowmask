use crate::diagnostics::BenchmarkOutcome;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkRun {
    pub segment: usize,
    pub start_ms: u64,
    pub duration_ms: u64,
    pub hardware: bool,
    pub outcome: BenchmarkOutcome,
    pub elapsed_ms: u64,
}

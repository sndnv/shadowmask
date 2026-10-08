use domain::diagnostics::{BenchmarkOutcome, BenchmarkRun};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BenchmarkRunResult {
    pub segment: usize,
    pub start_ms: u64,
    pub duration_ms: u64,
    pub encoder: &'static str,
    pub outcome: &'static str,
    pub elapsed_ms: u64,
    pub realtime: f64,
    pub detail: Option<String>,
}

impl From<BenchmarkRun> for BenchmarkRunResult {
    fn from(run: BenchmarkRun) -> Self {
        let (outcome, detail) = match run.outcome {
            BenchmarkOutcome::Ok => ("ok", None),
            BenchmarkOutcome::Failed { detail } => ("failed", Some(detail)),
            BenchmarkOutcome::TooSlow => ("too_slow", None),
        };
        let realtime = run.duration_ms as f64 / run.elapsed_ms.max(1) as f64;
        Self {
            segment: run.segment,
            start_ms: run.start_ms,
            duration_ms: run.duration_ms,
            encoder: if run.hardware { "vaapi" } else { "software" },
            outcome,
            elapsed_ms: run.elapsed_ms,
            realtime: (realtime * 100.0).round() / 100.0,
            detail,
        }
    }
}

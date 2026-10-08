#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BenchmarkOutcome {
    Ok,
    Failed { detail: String },
    TooSlow,
}

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BenchmarkProgress {
    pub done: usize,
    pub total: usize,
}

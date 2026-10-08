use crate::diagnostics::{BenchmarkStatus, BenchmarkTarget};
use crate::error::BenchmarkError;

pub trait TranscodeBenchmark: Send + Sync {
    fn status(&self) -> BenchmarkStatus;

    fn start(&self, target: BenchmarkTarget) -> Result<(), BenchmarkError>;
}

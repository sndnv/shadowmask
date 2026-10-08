use domain::diagnostics::BenchmarkTarget;
use serde::Serialize;

use crate::dto::catalog::HdrFormatDto;

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkSource {
    pub codec: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub hdr: Option<HdrFormatDto>,
    pub duration_ms: u64,
}

impl From<BenchmarkTarget> for BenchmarkSource {
    fn from(target: BenchmarkTarget) -> Self {
        Self {
            codec: target.codec,
            width: target.width,
            height: target.height,
            hdr: target.hdr.map(HdrFormatDto::from),
            duration_ms: target.duration_ms,
        }
    }
}

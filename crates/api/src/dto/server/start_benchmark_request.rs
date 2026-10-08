use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct StartBenchmarkRequest {
    pub version_id: String,
}

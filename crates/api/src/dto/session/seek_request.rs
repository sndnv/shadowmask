use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SeekRequest {
    pub position_ms: u64,
}

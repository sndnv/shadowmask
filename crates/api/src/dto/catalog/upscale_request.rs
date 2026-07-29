use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UpscaleRequest {
    pub target_height: u32,
}

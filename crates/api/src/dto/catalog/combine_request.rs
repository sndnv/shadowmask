use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CombineRequest {
    pub primary_subtitle_id: String,
    pub secondary_subtitle_id: String,
}

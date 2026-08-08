use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CombineRequest {
    pub top_subtitle_id: String,
    pub bottom_subtitle_id: String,
}

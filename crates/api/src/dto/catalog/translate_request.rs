use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TranslateRequest {
    pub source_subtitle_id: String,
    pub target_language: String,
}

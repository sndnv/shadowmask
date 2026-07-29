use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SubtitleTextResponse {
    pub content: String,
}

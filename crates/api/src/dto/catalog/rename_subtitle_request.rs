use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RenameSubtitleRequest {
    #[serde(default)]
    pub language: Option<String>,
}

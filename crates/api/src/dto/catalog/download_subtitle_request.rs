use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DownloadSubtitleRequest {
    pub file_id: String,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub release_name: Option<String>,
}

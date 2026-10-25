use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct SubmitReportRequest {
    pub player_id: Option<String>,
    pub app: String,
    pub platform: String,
    pub app_version: String,
    pub note: Option<String>,
    pub payload: Value,
}

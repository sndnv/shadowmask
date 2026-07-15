use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    #[serde(default, rename = "eventType")]
    pub event_type: Option<String>,
}

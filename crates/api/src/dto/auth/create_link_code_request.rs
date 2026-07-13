use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct CreateLinkCodeRequest {
    pub user_id: Option<String>,
    pub ttl_secs: Option<i64>,
}

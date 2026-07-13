use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

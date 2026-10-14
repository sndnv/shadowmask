use serde::{Deserialize, Serialize};

use super::TargetLocationResponse;

#[derive(Debug, Deserialize)]
pub struct PlayCommandRequest {
    pub version_id: String,
}

#[derive(Debug, Serialize)]
pub struct PlayCommandResponse {
    pub token: String,
    pub expires_at: String,
    pub target: TargetLocationResponse,
}

#[derive(Debug, Deserialize)]
pub struct RedeemCommandRequest {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct RedeemCommandResponse {
    pub version_id: String,
}

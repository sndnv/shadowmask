use jiff::Timestamp;

use crate::user::{DeviceId, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApiTokenId(pub String);

#[derive(Debug, Clone)]
pub struct ApiToken {
    pub id: ApiTokenId,
    pub user: UserId,
    pub device: DeviceId,
    pub token_hash: String,
    pub created_at: Timestamp,
    pub last_used_at: Option<Timestamp>,
}

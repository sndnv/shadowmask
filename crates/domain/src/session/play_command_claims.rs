use jiff::Timestamp;

use crate::catalog::VersionId;
use crate::user::{DeviceId, UserId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayCommandClaims {
    pub user: UserId,
    pub device: DeviceId,
    pub version: VersionId,
    pub expires_at: Timestamp,
    pub nonce: String,
}

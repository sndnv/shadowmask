use jiff::Timestamp;

use crate::catalog::VersionId;
use crate::user::UserId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadClaims {
    pub user: UserId,
    pub version: VersionId,
    pub expires_at: Timestamp,
    pub nonce: String,
}

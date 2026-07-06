use jiff::Timestamp;

use crate::catalog::VersionId;
use crate::session::SessionId;
use crate::user::UserId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamClaims {
    pub session: SessionId,
    pub user: UserId,
    pub version: VersionId,
    pub expires_at: Timestamp,
    pub nonce: String,
}

use jiff::Timestamp;

use crate::catalog::VersionId;
use crate::session::{SessionId, StreamGeneration};
use crate::user::UserId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamClaims {
    pub session: SessionId,
    pub generation: StreamGeneration,
    pub user: UserId,
    pub version: VersionId,
    pub expires_at: Timestamp,
    pub nonce: String,
}

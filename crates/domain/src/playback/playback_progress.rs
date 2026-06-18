use jiff::Timestamp;

use crate::catalog::VersionId;
use crate::user::UserId;

#[derive(Debug, Clone)]
pub struct PlaybackProgress {
    pub user: UserId,
    pub version: VersionId,
    pub position_ms: u64,
    pub updated_at: Timestamp,
}

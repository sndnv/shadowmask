use jiff::Timestamp;

use crate::user::{Role, UserId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingLink {
    pub code: String,
    pub user: UserId,
    pub role: Role,
    pub expires_at: Timestamp,
}

use jiff::Timestamp;

use crate::user::UserId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AuthSessionId(pub String);

#[derive(Debug, Clone)]
pub struct AuthSession {
    pub id: AuthSessionId,
    pub user: UserId,
    pub refresh_token_hash: String,
    pub issued_at: Timestamp,
    pub expires_at: Timestamp,
}

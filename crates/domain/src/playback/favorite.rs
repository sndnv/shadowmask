use jiff::Timestamp;

use crate::catalog::TitleId;
use crate::user::UserId;

#[derive(Debug, Clone)]
pub struct Favorite {
    pub user: UserId,
    pub title: TitleId,
    pub added_at: Timestamp,
}

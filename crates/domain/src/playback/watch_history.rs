use jiff::Timestamp;

use crate::catalog::TitleId;
use crate::user::UserId;

#[derive(Debug, Clone)]
pub struct WatchHistory {
    pub user: UserId,
    pub title: TitleId,
    pub watched: bool,
    pub play_count: u32,
    pub last_watched_at: Option<Timestamp>,
    pub completed: bool,
}

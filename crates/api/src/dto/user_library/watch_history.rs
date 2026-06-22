use serde::Serialize;

use domain::playback::WatchHistory;

use crate::dto::common::TitleRefDto;

#[derive(Debug, Serialize)]
pub struct WatchHistoryResponse {
    pub user_id: String,
    pub title: TitleRefDto,
    pub watched: bool,
    pub play_count: u32,
    pub last_watched_at: Option<String>,
    pub completed: bool,
}

impl From<WatchHistory> for WatchHistoryResponse {
    fn from(h: WatchHistory) -> Self {
        WatchHistoryResponse {
            user_id: h.user.0,
            title: h.title.into(),
            watched: h.watched,
            play_count: h.play_count,
            last_watched_at: h.last_watched_at.map(|t| t.to_string()),
            completed: h.completed,
        }
    }
}

use serde::Serialize;

use domain::playback::WatchlistItem;

use crate::dto::common::TitleRefDto;

#[derive(Debug, Serialize)]
pub struct WatchlistItemResponse {
    pub user_id: String,
    pub title: TitleRefDto,
    pub added_at: String,
}

impl From<WatchlistItem> for WatchlistItemResponse {
    fn from(w: WatchlistItem) -> Self {
        WatchlistItemResponse {
            user_id: w.user.0,
            title: w.title.into(),
            added_at: w.added_at.to_string(),
        }
    }
}

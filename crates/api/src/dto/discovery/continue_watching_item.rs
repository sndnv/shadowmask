use serde::Serialize;

use domain::discovery::ContinueWatchingItem;

use crate::dto::user_library::PlaybackProgressResponse;

#[derive(Debug, Serialize)]
pub struct ContinueWatchingItemResponse {
    pub progress: PlaybackProgressResponse,
}

impl From<ContinueWatchingItem> for ContinueWatchingItemResponse {
    fn from(c: ContinueWatchingItem) -> Self {
        ContinueWatchingItemResponse {
            progress: c.progress.into(),
        }
    }
}

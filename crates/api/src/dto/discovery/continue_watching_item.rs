use serde::Serialize;

use domain::discovery::ContinueWatchingItem;

use crate::dto::common::ResumeCardDto;
use crate::dto::user_library::PlaybackProgressResponse;

#[derive(Debug, Serialize)]
pub struct ContinueWatchingItemResponse {
    pub progress: PlaybackProgressResponse,
    pub card: ResumeCardDto,
}

impl From<ContinueWatchingItem> for ContinueWatchingItemResponse {
    fn from(c: ContinueWatchingItem) -> Self {
        ContinueWatchingItemResponse { progress: c.progress.into(), card: c.card.into() }
    }
}

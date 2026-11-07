use serde::Serialize;

use domain::playback::{PlaylistCard, PlaylistDetail};

use crate::dto::catalog::TitleCardResponse;
use crate::dto::playlist::PlaylistResponse;

#[derive(Debug, Serialize)]
pub struct PlaylistItemResponse {
    pub position: u32,
    pub added_at: String,
    pub card: TitleCardResponse,
}

impl From<PlaylistCard> for PlaylistItemResponse {
    fn from(c: PlaylistCard) -> Self {
        PlaylistItemResponse {
            position: c.item.position,
            added_at: c.item.added_at.to_string(),
            card: c.card.into(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PlaylistDetailResponse {
    #[serde(flatten)]
    pub playlist: PlaylistResponse,
    pub items: Vec<PlaylistItemResponse>,
}

impl From<PlaylistDetail> for PlaylistDetailResponse {
    fn from(d: PlaylistDetail) -> Self {
        PlaylistDetailResponse {
            playlist: d.playlist.into(),
            items: d.items.into_iter().map(Into::into).collect(),
        }
    }
}

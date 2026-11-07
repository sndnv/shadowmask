use serde::Serialize;

use domain::playback::{Playlist, PlaylistSummary};

#[derive(Debug, Serialize)]
pub struct PlaylistResponse {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Playlist> for PlaylistResponse {
    fn from(p: Playlist) -> Self {
        PlaylistResponse {
            id: p.id.0,
            name: p.name,
            created_at: p.created_at.to_string(),
            updated_at: p.updated_at.to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PlaylistSummaryResponse {
    #[serde(flatten)]
    pub playlist: PlaylistResponse,
    pub item_count: usize,
}

impl From<PlaylistSummary> for PlaylistSummaryResponse {
    fn from(s: PlaylistSummary) -> Self {
        PlaylistSummaryResponse { playlist: s.playlist.into(), item_count: s.item_count }
    }
}

use serde::Serialize;

use domain::playback::PlaybackProgress;

#[derive(Debug, Serialize)]
pub struct PlaybackProgressResponse {
    pub user_id: String,
    pub version_id: String,
    pub position_ms: u64,
    pub updated_at: String,
}

impl From<PlaybackProgress> for PlaybackProgressResponse {
    fn from(p: PlaybackProgress) -> Self {
        PlaybackProgressResponse {
            user_id: p.user.0,
            version_id: p.version.0,
            position_ms: p.position_ms,
            updated_at: p.updated_at.to_string(),
        }
    }
}

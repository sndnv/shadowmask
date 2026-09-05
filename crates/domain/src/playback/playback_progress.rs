use jiff::Timestamp;

use crate::catalog::VersionId;
use crate::playback::SubtitleOverride;
use crate::user::UserId;

#[derive(Debug, Clone)]
pub struct PlaybackProgress {
    pub user: UserId,
    pub version: VersionId,
    pub position_ms: u64,
    pub audio_track: Option<u32>,
    pub subtitle: Option<SubtitleOverride>,
    pub updated_at: Timestamp,
}

impl PlaybackProgress {
    pub fn has_override(&self) -> bool {
        self.audio_track.is_some() || self.subtitle.is_some()
    }
}

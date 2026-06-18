use crate::catalog::VersionId;
use crate::playback::SubtitleTrackRef;
use crate::user::UserId;

#[derive(Debug, Clone)]
pub struct UserSubtitleOffset {
    pub user: UserId,
    pub version: VersionId,
    pub subtitle: SubtitleTrackRef,
    pub offset_ms: i64,
}

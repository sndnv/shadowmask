use jiff::Timestamp;

use crate::catalog::VersionId;
use crate::session::{DeliveryMode, PlaybackState, SelectedTracks};
use crate::user::{DeviceId, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);

#[derive(Debug, Clone)]
pub struct PlaybackSession {
    pub id: SessionId,
    pub user: UserId,
    pub device: Option<DeviceId>,
    pub version: VersionId,
    pub mode: DeliveryMode,
    pub position_ms: u64,
    pub state: PlaybackState,
    pub selected: SelectedTracks,
    pub started_at: Timestamp,
    pub last_heartbeat_at: Timestamp,
    pub completed: bool,
}

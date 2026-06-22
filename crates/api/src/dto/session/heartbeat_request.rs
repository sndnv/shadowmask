use serde::Deserialize;

use super::PlaybackStateDto;

#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    pub position_ms: u64,
    pub state: PlaybackStateDto,
}

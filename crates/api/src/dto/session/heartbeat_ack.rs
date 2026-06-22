use serde::Serialize;

use domain::session::HeartbeatAck;

#[derive(Debug, Serialize)]
pub struct HeartbeatAckResponse {
    pub heartbeat_interval_s: u32,
}

impl From<HeartbeatAck> for HeartbeatAckResponse {
    fn from(a: HeartbeatAck) -> Self {
        HeartbeatAckResponse {
            heartbeat_interval_s: a.heartbeat_interval_s,
        }
    }
}

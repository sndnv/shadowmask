use jiff::Timestamp;

use super::DeviceId;

#[derive(Debug, Clone)]
pub struct IssuedToken {
    pub token: String,
    pub expires_at: Option<Timestamp>,
    pub device: DeviceId,
}

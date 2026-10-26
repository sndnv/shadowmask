use jiff::Timestamp;

use crate::user::{DeviceId, UserId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceShare {
    pub device: DeviceId,
    pub user: UserId,
    pub created_at: Timestamp,
}

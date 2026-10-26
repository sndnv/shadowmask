use crate::user::{DeviceId, UserId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedWith {
    pub device: DeviceId,
    pub user: UserId,
    pub username: String,
}

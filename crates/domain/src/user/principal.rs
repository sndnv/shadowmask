use crate::user::{DeviceId, Role, UserId};

#[derive(Debug, Clone)]
pub struct Principal {
    pub user: UserId,
    pub role: Role,
    pub account_admin: bool,
    pub device: Option<DeviceId>,
}

use jiff::Timestamp;

use crate::user::UserId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceId(pub String);

#[derive(Debug, Clone)]
pub struct Device {
    pub id: DeviceId,
    pub user: UserId,
    pub name: String,
    pub platform: String,
    pub created_at: Timestamp,
    pub last_seen: Option<Timestamp>,
}

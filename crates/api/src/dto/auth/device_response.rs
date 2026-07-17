use serde::Serialize;

use domain::user::Device;

#[derive(Debug, Serialize)]
pub struct DeviceResponse {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub platform: String,
    pub created_at: String,
    pub last_seen: Option<String>,
}

impl From<Device> for DeviceResponse {
    fn from(d: Device) -> Self {
        DeviceResponse {
            id: d.id.0,
            user_id: d.user.0,
            name: d.name,
            platform: d.platform,
            created_at: d.created_at.to_string(),
            last_seen: d.last_seen.map(|t| t.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::user::{DeviceId, UserId};
    use jiff::Timestamp;

    #[test]
    fn maps_device_fields() {
        let response = DeviceResponse::from(Device {
            id: DeviceId("d1".into()),
            user: UserId("u1".into()),
            name: "Roku".into(),
            platform: "roku".into(),
            created_at: Timestamp::UNIX_EPOCH,
            last_seen: Some(Timestamp::UNIX_EPOCH),
        });
        assert_eq!(response.id, "d1");
        assert_eq!(response.user_id, "u1");
        assert_eq!(response.name, "Roku");
        assert_eq!(response.platform, "roku");
        assert_eq!(response.created_at, Timestamp::UNIX_EPOCH.to_string());
        assert!(response.last_seen.is_some());

        let never_seen = DeviceResponse::from(Device {
            id: DeviceId("d2".into()),
            user: UserId("u1".into()),
            name: "Web".into(),
            platform: "web".into(),
            created_at: Timestamp::UNIX_EPOCH,
            last_seen: None,
        });
        assert!(never_seen.last_seen.is_none());
    }
}

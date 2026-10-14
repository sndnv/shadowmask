use serde::Serialize;

use domain::user::Device;

use crate::dto::play_on::DeviceTargetResponse;

#[derive(Debug, Serialize)]
pub struct DeviceResponse {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub platform: String,
    pub created_at: String,
    pub last_seen: Option<String>,
    pub target: Option<DeviceTargetResponse>,
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
            target: d.target.map(DeviceTargetResponse::from),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::user::{DeviceId, DeviceTarget, UserId};
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
            target: None,
        });
        assert_eq!(response.id, "d1");
        assert_eq!(response.user_id, "u1");
        assert_eq!(response.name, "Roku");
        assert_eq!(response.platform, "roku");
        assert_eq!(response.created_at, Timestamp::UNIX_EPOCH.to_string());
        assert!(response.last_seen.is_some());
        assert!(response.target.is_none());

        let never_seen = DeviceResponse::from(Device {
            id: DeviceId("d2".into()),
            user: UserId("u1".into()),
            name: "Web".into(),
            platform: "web".into(),
            created_at: Timestamp::UNIX_EPOCH,
            last_seen: None,
            target: None,
        });
        assert!(never_seen.last_seen.is_none());
    }

    #[test]
    fn a_device_without_a_target_says_null_and_one_with_a_target_carries_it() {
        let device = |target| Device {
            id: DeviceId("d1".into()),
            user: UserId("u1".into()),
            name: "Roku".into(),
            platform: "roku".into(),
            created_at: Timestamp::UNIX_EPOCH,
            last_seen: None,
            target,
        };
        let without = serde_json::to_value(DeviceResponse::from(device(None))).unwrap();
        assert!(without["target"].is_null());

        let target = DeviceTarget::new(
            "roku",
            &["192.168.1.40".to_owned()],
            8060,
            Some("dev".into()),
            Timestamp::UNIX_EPOCH,
        )
        .unwrap();
        let with = serde_json::to_value(DeviceResponse::from(device(Some(target)))).unwrap();
        assert_eq!(with["target"]["addresses"][0], "192.168.1.40");
        assert_eq!(with["target"]["reported_at"], "1970-01-01T00:00:00Z");
    }
}

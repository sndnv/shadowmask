use serde::Serialize;

use domain::user::SharedTarget;

use crate::dto::play_on::DeviceTargetResponse;

#[derive(Debug, Serialize)]
pub struct SharedTargetResponse {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub owner: String,
    pub target: Option<DeviceTargetResponse>,
}

impl From<SharedTarget> for SharedTargetResponse {
    fn from(s: SharedTarget) -> Self {
        SharedTargetResponse {
            id: s.device.id.0,
            name: s.device.name,
            platform: s.device.platform,
            owner: s.owner,
            target: s.device.target.map(DeviceTargetResponse::from),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::user::{Device, DeviceId, DeviceTarget, UserId};
    use jiff::Timestamp;
    use serde_json::json;

    #[test]
    fn a_shared_target_names_its_owner_and_leaves_out_the_owners_id() {
        let target = DeviceTarget::new(
            "roku",
            &["192.168.1.40".to_owned()],
            8060,
            Some("dev".into()),
            Timestamp::UNIX_EPOCH,
        )
        .unwrap();
        let response = SharedTargetResponse::from(SharedTarget {
            device: Device {
                id: DeviceId("roku-1".into()),
                user: UserId("u1".into()),
                name: "Living room".into(),
                platform: "roku".into(),
                created_at: Timestamp::UNIX_EPOCH,
                last_seen: None,
                target: Some(target),
            },
            owner: "user".into(),
        });
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            json!({
                "id": "roku-1",
                "name": "Living room",
                "platform": "roku",
                "owner": "user",
                "target": {
                    "kind": "roku",
                    "addresses": ["192.168.1.40"],
                    "port": 8060,
                    "app_id": "dev",
                    "reported_at": "1970-01-01T00:00:00Z"
                }
            })
        );
    }
}

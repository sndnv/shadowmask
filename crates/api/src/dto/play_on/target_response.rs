use serde::Serialize;

use domain::user::DeviceTarget;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct TargetLocationResponse {
    pub kind: &'static str,
    pub addresses: Vec<String>,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,
}

impl From<&DeviceTarget> for TargetLocationResponse {
    fn from(t: &DeviceTarget) -> Self {
        TargetLocationResponse {
            kind: t.kind.as_str(),
            addresses: t.addresses.iter().map(ToString::to_string).collect(),
            port: t.port,
            app_id: t.app_id.clone(),
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct DeviceTargetResponse {
    #[serde(flatten)]
    pub location: TargetLocationResponse,
    pub reported_at: String,
}

impl From<DeviceTarget> for DeviceTargetResponse {
    fn from(t: DeviceTarget) -> Self {
        DeviceTargetResponse {
            location: TargetLocationResponse::from(&t),
            reported_at: t.reported_at.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::Timestamp;
    use serde_json::json;

    fn target(kind: &str, app_id: Option<&str>) -> DeviceTarget {
        DeviceTarget::new(
            kind,
            &["192.168.1.40".to_owned(), "10.0.0.2".to_owned()],
            8060,
            app_id.map(str::to_owned),
            Timestamp::UNIX_EPOCH,
        )
        .unwrap()
    }

    #[test]
    fn a_roku_report_serializes_with_its_app_id_and_time() {
        let json =
            serde_json::to_value(DeviceTargetResponse::from(target("roku", Some("dev")))).unwrap();
        assert_eq!(
            json,
            json!({
                "kind": "roku",
                "addresses": ["192.168.1.40", "10.0.0.2"],
                "port": 8060,
                "app_id": "dev",
                "reported_at": "1970-01-01T00:00:00Z"
            })
        );
    }

    #[test]
    fn an_android_location_has_no_app_id_field() {
        let json =
            serde_json::to_value(TargetLocationResponse::from(&target("android", None))).unwrap();
        assert_eq!(
            json,
            json!({ "kind": "android", "addresses": ["192.168.1.40", "10.0.0.2"], "port": 8060 })
        );
    }
}

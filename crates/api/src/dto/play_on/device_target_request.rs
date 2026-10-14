use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DeviceTargetRequest {
    pub kind: String,
    pub addresses: Vec<String>,
    pub port: i64,
    pub app_id: Option<String>,
}

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ShareDeviceRequest {
    pub username: String,
}

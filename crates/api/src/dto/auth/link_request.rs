use serde::Deserialize;

use super::DeviceRegistrationDto;

#[derive(Debug, Deserialize)]
pub struct LinkRequest {
    pub code: String,
    pub device: DeviceRegistrationDto,
}

use serde::Deserialize;

use domain::user::DeviceRegistration;

#[derive(Debug, Deserialize)]
pub struct DeviceRegistrationDto {
    pub name: String,
    pub platform: String,
}

impl From<DeviceRegistrationDto> for DeviceRegistration {
    fn from(d: DeviceRegistrationDto) -> Self {
        DeviceRegistration { name: d.name, platform: d.platform }
    }
}

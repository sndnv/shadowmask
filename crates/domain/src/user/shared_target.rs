use crate::user::Device;

#[derive(Debug, Clone)]
pub struct SharedTarget {
    pub device: Device,
    pub owner: String,
}

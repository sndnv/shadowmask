use crate::user::{Device, Principal};

#[derive(Debug, Clone)]
pub struct CommandTarget {
    pub device: Device,
    pub plays_as: Principal,
}

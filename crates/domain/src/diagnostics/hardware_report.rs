use crate::diagnostics::{AccelerationMode, HardwareTest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareReport {
    pub mode: AccelerationMode,
    pub device: String,
    pub device_present: bool,
    pub in_use: bool,
    pub test: HardwareTest,
}

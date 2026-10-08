use domain::diagnostics::HardwareReport;
use serde::Serialize;

use crate::dto::server::HardwareTestResult;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HardwareCapabilities {
    pub mode: &'static str,
    pub device: String,
    pub device_present: bool,
    pub in_use: bool,
    pub test: HardwareTestResult,
}

impl From<HardwareReport> for HardwareCapabilities {
    fn from(hardware: HardwareReport) -> Self {
        Self {
            mode: hardware.mode.as_str(),
            device: hardware.device,
            device_present: hardware.device_present,
            in_use: hardware.in_use,
            test: hardware.test.into(),
        }
    }
}

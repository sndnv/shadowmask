use domain::diagnostics::HardwareTest;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HardwareTestResult {
    pub outcome: &'static str,
    pub low_power: Option<bool>,
    pub elapsed_ms: Option<u64>,
    pub detail: Option<String>,
    pub low_power_detail: Option<String>,
}

impl From<HardwareTest> for HardwareTestResult {
    fn from(test: HardwareTest) -> Self {
        let empty = Self {
            outcome: "skipped",
            low_power: None,
            elapsed_ms: None,
            detail: None,
            low_power_detail: None,
        };
        match test {
            HardwareTest::Skipped { reason } => Self { detail: Some(reason), ..empty },
            HardwareTest::Works { low_power, elapsed_ms } => Self {
                outcome: "works",
                low_power: Some(low_power),
                elapsed_ms: Some(elapsed_ms),
                ..empty
            },
            HardwareTest::Failed { detail, low_power_detail } => Self {
                outcome: "failed",
                detail: Some(detail),
                low_power_detail: Some(low_power_detail),
                ..empty
            },
        }
    }
}

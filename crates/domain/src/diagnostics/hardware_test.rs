#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardwareTest {
    Skipped { reason: String },
    Works { low_power: bool, elapsed_ms: u64 },
    Failed { detail: String, low_power_detail: String },
}

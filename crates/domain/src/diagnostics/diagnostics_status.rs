use jiff::Timestamp;

use crate::diagnostics::CapabilityReport;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiagnosticsStatus {
    pub checking: bool,
    pub checked_at: Option<Timestamp>,
    pub report: Option<CapabilityReport>,
}

use crate::diagnostics::{FfmpegReport, HardwareReport, HostReport};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityReport {
    pub host: HostReport,
    pub ffmpeg: FfmpegReport,
    pub hardware: HardwareReport,
}

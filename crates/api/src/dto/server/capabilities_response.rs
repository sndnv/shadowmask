use domain::diagnostics::DiagnosticsStatus;
use serde::Serialize;

use crate::dto::server::{FfmpegCapabilities, HardwareCapabilities, HostCapabilities};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapabilitiesResponse {
    pub state: &'static str,
    pub checked_at: Option<String>,
    pub host: Option<HostCapabilities>,
    pub ffmpeg: Option<FfmpegCapabilities>,
    pub hardware: Option<HardwareCapabilities>,
}

impl From<DiagnosticsStatus> for CapabilitiesResponse {
    fn from(status: DiagnosticsStatus) -> Self {
        let state = match (status.checking, &status.report) {
            (true, _) => "checking",
            (false, Some(_)) => "ready",
            (false, None) => "unchecked",
        };
        let checked_at = status.checked_at.map(|at| at.to_string());
        match status.report {
            Some(report) => Self {
                state,
                checked_at,
                host: Some(report.host.into()),
                ffmpeg: Some(report.ffmpeg.into()),
                hardware: Some(report.hardware.into()),
            },
            None => Self { state, checked_at, host: None, ffmpeg: None, hardware: None },
        }
    }
}

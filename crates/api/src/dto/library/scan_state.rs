use serde::Serialize;

use domain::library::{ScanState, ScanStatus};

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanStatusDto {
    Idle,
    Running,
    Failed,
}

impl From<ScanStatus> for ScanStatusDto {
    fn from(s: ScanStatus) -> Self {
        match s {
            ScanStatus::Idle => ScanStatusDto::Idle,
            ScanStatus::Running => ScanStatusDto::Running,
            ScanStatus::Failed => ScanStatusDto::Failed,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ScanStateResponse {
    pub library_id: String,
    pub status: ScanStatusDto,
    pub progress: f32,
    pub last_scanned_at: Option<String>,
    pub error: Option<String>,
}

impl From<ScanState> for ScanStateResponse {
    fn from(s: ScanState) -> Self {
        ScanStateResponse {
            library_id: s.library.0,
            status: s.status.into(),
            progress: s.progress,
            last_scanned_at: s.last_scanned_at.map(|t| t.to_string()),
            error: s.error,
        }
    }
}

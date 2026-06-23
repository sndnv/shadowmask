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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_all_statuses() {
        assert!(matches!(
            ScanStatusDto::from(ScanStatus::Idle),
            ScanStatusDto::Idle
        ));
        assert!(matches!(
            ScanStatusDto::from(ScanStatus::Running),
            ScanStatusDto::Running
        ));
        assert!(matches!(
            ScanStatusDto::from(ScanStatus::Failed),
            ScanStatusDto::Failed
        ));
    }

    #[test]
    fn maps_state_with_timestamp() {
        use domain::library::LibraryId;
        use jiff::Timestamp;

        let ts: Timestamp = "2024-01-01T00:00:00Z".parse().unwrap();
        let resp = ScanStateResponse::from(ScanState {
            library: LibraryId("l1".into()),
            status: ScanStatus::Idle,
            progress: 0.5,
            last_scanned_at: Some(ts),
            error: Some("boom".into()),
        });
        assert_eq!(resp.library_id, "l1");
        assert_eq!(resp.last_scanned_at, Some(ts.to_string()));
        assert_eq!(resp.error.as_deref(), Some("boom"));
    }
}

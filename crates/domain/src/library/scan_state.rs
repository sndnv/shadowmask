use jiff::Timestamp;

use crate::library::LibraryId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScanStatus {
    Idle,
    Queued,
    Running,
    Failed,
    Purging,
    Cleaning,
}

impl ScanStatus {
    pub fn is_busy(self) -> bool {
        matches!(
            self,
            ScanStatus::Queued | ScanStatus::Running | ScanStatus::Purging | ScanStatus::Cleaning
        )
    }
}

#[derive(Debug, Clone)]
pub struct ScanState {
    pub library: LibraryId,
    pub status: ScanStatus,
    pub progress: f32,
    pub started_at: Option<Timestamp>,
    pub last_scanned_at: Option<Timestamp>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_library_is_busy_while_a_scan_a_purge_or_a_clean_holds_it() {
        for status in
            [ScanStatus::Queued, ScanStatus::Running, ScanStatus::Purging, ScanStatus::Cleaning]
        {
            assert!(status.is_busy(), "{status:?}");
        }
        for status in [ScanStatus::Idle, ScanStatus::Failed] {
            assert!(!status.is_busy(), "{status:?}");
        }
    }
}

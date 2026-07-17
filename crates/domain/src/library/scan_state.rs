use jiff::Timestamp;

use crate::library::LibraryId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScanStatus {
    Idle,
    Queued,
    Running,
    Failed,
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

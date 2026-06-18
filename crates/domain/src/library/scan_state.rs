use jiff::Timestamp;

use crate::library::LibraryId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScanStatus {
    Idle,
    Running,
    Failed,
}

#[derive(Debug, Clone)]
pub struct ScanState {
    pub library: LibraryId,
    pub status: ScanStatus,
    pub progress: f32,
    pub last_scanned_at: Option<Timestamp>,
    pub error: Option<String>,
}

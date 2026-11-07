use jiff::Timestamp;

use crate::catalog::TitleId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistItem {
    pub position: u32,
    pub title: TitleId,
    pub added_at: Timestamp,
}

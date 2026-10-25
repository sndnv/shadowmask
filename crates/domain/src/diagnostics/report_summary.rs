use jiff::Timestamp;

use crate::diagnostics::{PlayerId, ReportId};
use crate::user::UserId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportSummary {
    pub id: ReportId,
    pub created_at: Timestamp,
    pub user: UserId,
    pub player: PlayerId,
    pub app: String,
    pub platform: String,
    pub app_version: String,
    pub note: Option<String>,
}

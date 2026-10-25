use domain::diagnostics::ReportSummary;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReportSummaryResponse {
    pub id: String,
    pub created_at: String,
    pub user_id: String,
    pub player_id: String,
    pub app: String,
    pub platform: String,
    pub app_version: String,
    pub note: Option<String>,
}

impl From<ReportSummary> for ReportSummaryResponse {
    fn from(summary: ReportSummary) -> Self {
        Self {
            id: summary.id.0,
            created_at: summary.created_at.to_string(),
            user_id: summary.user.0,
            player_id: summary.player.0,
            app: summary.app,
            platform: summary.platform,
            app_version: summary.app_version,
            note: summary.note,
        }
    }
}

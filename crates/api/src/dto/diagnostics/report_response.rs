use domain::diagnostics::Report;
use serde::Serialize;
use serde_json::Value;

use crate::dto::diagnostics::ReportSummaryResponse;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReportResponse {
    #[serde(flatten)]
    pub summary: ReportSummaryResponse,
    pub payload: Value,
}

impl From<Report> for ReportResponse {
    fn from(report: Report) -> Self {
        let payload =
            serde_json::from_str(&report.payload).unwrap_or(Value::String(report.payload));
        Self { summary: report.summary.into(), payload }
    }
}

#[cfg(test)]
mod tests {
    use domain::diagnostics::{PlayerId, ReportId, ReportSummary};
    use domain::user::UserId;
    use jiff::Timestamp;
    use serde_json::json;

    use super::*;

    fn report(payload: &str) -> Report {
        Report {
            summary: ReportSummary {
                id: ReportId("r1".into()),
                created_at: Timestamp::UNIX_EPOCH,
                user: UserId("u1".into()),
                player: PlayerId("p1".into()),
                app: "flutter".into(),
                platform: "Android 14".into(),
                app_version: "0.0.7".into(),
                note: Some("stutters".into()),
            },
            payload: payload.into(),
        }
    }

    #[test]
    fn the_summary_sits_beside_the_payload_as_json() {
        let body = serde_json::to_value(ReportResponse::from(report("{\"schema\":1}"))).unwrap();
        assert_eq!(
            body,
            json!({
                "id": "r1",
                "created_at": "1970-01-01T00:00:00Z",
                "user_id": "u1",
                "player_id": "p1",
                "app": "flutter",
                "platform": "Android 14",
                "app_version": "0.0.7",
                "note": "stutters",
                "payload": { "schema": 1 }
            })
        );
    }

    #[test]
    fn a_stored_payload_that_is_not_json_is_shown_as_text() {
        let response = ReportResponse::from(report("not json"));
        assert_eq!(response.payload, json!("not json"));
    }
}

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReportCreatedResponse {
    pub id: String,
}

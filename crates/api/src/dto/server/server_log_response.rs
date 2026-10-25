use domain::diagnostics::ServerLogPage;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServerLogResponse {
    pub entries: Vec<String>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub more: bool,
}

impl From<ServerLogPage> for ServerLogResponse {
    fn from(page: ServerLogPage) -> Self {
        Self {
            entries: page.entries,
            before: page.before.map(|cursor| cursor.to_string()),
            after: page.after.map(|cursor| cursor.to_string()),
            more: page.more,
        }
    }
}

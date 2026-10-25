use crate::diagnostics::ServerLogCursor;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServerLogPage {
    pub entries: Vec<String>,
    pub before: Option<ServerLogCursor>,
    pub after: Option<ServerLogCursor>,
    pub more: bool,
}

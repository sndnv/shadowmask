use crate::common::LogLevel;
use crate::diagnostics::ServerLogFrom;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerLogRequest {
    pub from: ServerLogFrom,
    pub limit: usize,
    pub min_level: Option<LogLevel>,
}

use crate::diagnostics::ReportSummary;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReportId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub summary: ReportSummary,
    pub payload: String,
}

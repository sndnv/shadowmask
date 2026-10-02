#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    ProbeFailed(String),
    ExcludedDirectory(String),
}

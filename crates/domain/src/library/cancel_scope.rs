#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelScope {
    All,
    PurgeBlockers,
    CleanBlockers,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CancelledJobs {
    pub cancelled: u64,
    pub signalled: u64,
    pub running: u64,
}

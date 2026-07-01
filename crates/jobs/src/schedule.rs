use domain::job::{JobKind, JobPriority};
use jiff::{SignedDuration, Timestamp};

#[derive(Debug, Clone)]
pub struct Schedule {
    pub kind: JobKind,
    pub priority: JobPriority,
    pub payload: String,
    pub every: SignedDuration,
    pub next_fire_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchPlan {
    FsEvents {
        roots: Vec<String>,
    },
    Poll {
        roots: Vec<String>,
    },
    Cron {
        expression: String,
        roots: Vec<String>,
    },
    Manual,
}

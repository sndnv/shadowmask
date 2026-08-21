use crate::job::Job;

#[derive(Debug, Clone)]
pub struct JobNode {
    pub job: Job,
    pub depth: u32,
}

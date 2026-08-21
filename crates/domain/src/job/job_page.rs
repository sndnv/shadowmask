use crate::common::Page;
use crate::job::Job;

#[derive(Debug, Clone)]
pub struct JobPage {
    pub page: Page<Job>,
    pub active_total: u64,
    pub all_total: u64,
}

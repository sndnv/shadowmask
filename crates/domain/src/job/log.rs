use std::future::Future;

use jiff::Timestamp;

use crate::common::LogLevel;
use crate::error::JobLogError;
use crate::job::JobId;

pub trait JobLogStore: Send + Sync {
    fn append(
        &self,
        job: &JobId,
        at: Timestamp,
        level: LogLevel,
        message: &str,
    ) -> impl Future<Output = Result<(), JobLogError>> + Send;
    fn read(
        &self,
        job: &JobId,
        tail: Option<usize>,
    ) -> impl Future<Output = Result<Vec<String>, JobLogError>> + Send;
    fn wipe(&self, job: &JobId) -> impl Future<Output = Result<(), JobLogError>> + Send;
}

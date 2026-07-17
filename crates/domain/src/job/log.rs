use std::future::Future;

use jiff::Timestamp;

use crate::error::JobLogError;
use crate::job::JobId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl JobLogLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            JobLogLevel::Debug => "DEBUG",
            JobLogLevel::Info => "INFO",
            JobLogLevel::Warn => "WARN",
            JobLogLevel::Error => "ERROR",
        }
    }

    pub fn parse(token: &str) -> Option<Self> {
        [
            JobLogLevel::Debug,
            JobLogLevel::Info,
            JobLogLevel::Warn,
            JobLogLevel::Error,
        ]
        .into_iter()
        .find(|level| token.eq_ignore_ascii_case(level.as_str()))
    }
}

pub trait JobLogStore: Send + Sync {
    fn append(
        &self,
        job: &JobId,
        at: Timestamp,
        level: JobLogLevel,
        message: &str,
    ) -> impl Future<Output = Result<(), JobLogError>> + Send;
    fn read(
        &self,
        job: &JobId,
        tail: Option<usize>,
    ) -> impl Future<Output = Result<Vec<String>, JobLogError>> + Send;
    fn wipe(&self, job: &JobId) -> impl Future<Output = Result<(), JobLogError>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_round_trips_and_orders() {
        for level in [
            JobLogLevel::Debug,
            JobLogLevel::Info,
            JobLogLevel::Warn,
            JobLogLevel::Error,
        ] {
            assert_eq!(JobLogLevel::parse(level.as_str()), Some(level));
        }
        assert_eq!(JobLogLevel::parse("info"), Some(JobLogLevel::Info));
        assert_eq!(JobLogLevel::parse("verbose"), None);
        assert!(JobLogLevel::Debug < JobLogLevel::Info);
        assert!(JobLogLevel::Info < JobLogLevel::Warn);
        assert!(JobLogLevel::Warn < JobLogLevel::Error);
    }
}

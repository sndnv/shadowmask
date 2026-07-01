use thiserror::Error;

#[derive(Debug, Error)]
pub enum JobError {
    #[error("retryable job failure: {0}")]
    Retryable(String),
    #[error("permanent job failure: {0}")]
    Permanent(String),
}

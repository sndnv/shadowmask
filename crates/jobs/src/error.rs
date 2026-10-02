use domain::error::SubtitleError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum JobError {
    #[error("retryable job failure: {0}")]
    Retryable(String),
    #[error("permanent job failure: {0}")]
    Permanent(String),
}

impl From<SubtitleError> for JobError {
    fn from(err: SubtitleError) -> Self {
        let message = err.to_string();
        match err {
            SubtitleError::Refused(_) => JobError::Permanent(message),
            _ => JobError::Retryable(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_refusal_is_permanent() {
        let classified = [
            SubtitleError::Backend("boom".into()),
            SubtitleError::Parse("boom".into()),
            SubtitleError::NotFound,
            SubtitleError::Store("disk full".into()),
            SubtitleError::Refused("outside the store".into()),
        ]
        .map(JobError::from);

        assert!(matches!(classified[4], JobError::Permanent(_)));
        for failure in &classified[..4] {
            assert!(matches!(failure, JobError::Retryable(_)), "{failure:?}");
        }
    }
}

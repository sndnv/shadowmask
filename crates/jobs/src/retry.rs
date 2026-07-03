use domain::job::{Job, JobStatus};
use jiff::{SignedDuration, Timestamp};

use crate::error::JobError;

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay: SignedDuration,
    pub multiplier: u32,
    pub max_delay: SignedDuration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: SignedDuration::from_secs(1),
            multiplier: 2,
            max_delay: SignedDuration::from_secs(60),
        }
    }
}

pub fn backoff(attempts: u32, policy: &RetryPolicy) -> SignedDuration {
    let exponent = attempts.saturating_sub(1);
    let factor = policy.multiplier.checked_pow(exponent).unwrap_or(u32::MAX);
    let delay_ms = (policy.base_delay.as_millis() as i64).saturating_mul(i64::from(factor));
    let capped = delay_ms.min(policy.max_delay.as_millis() as i64);
    SignedDuration::from_millis(capped)
}

pub fn apply_outcome(
    mut job: Job,
    result: Result<(), JobError>,
    now: Timestamp,
    policy: &RetryPolicy,
) -> Job {
    job.attempts += 1;
    job.updated_at = now;
    match result {
        Ok(()) => {
            job.status = JobStatus::Succeeded;
            job.progress = 1.0;
            job.last_error = None;
        }
        Err(err) => {
            let retryable = matches!(err, JobError::Retryable(_));
            job.last_error = Some(err.to_string());
            if retryable && job.attempts < policy.max_attempts {
                job.status = JobStatus::Queued;
                job.available_at = now
                    .saturating_add(backoff(job.attempts, policy))
                    .unwrap_or(now);
            } else {
                job.status = JobStatus::Failed;
            }
        }
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::job::{JobId, JobKind, JobPriority};

    fn policy() -> RetryPolicy {
        RetryPolicy {
            max_attempts: 3,
            base_delay: SignedDuration::from_secs(1),
            multiplier: 2,
            max_delay: SignedDuration::from_secs(60),
        }
    }

    fn job(now: Timestamp) -> Job {
        Job {
            id: JobId("a".into()),
            kind: JobKind::LibraryScan,
            status: JobStatus::Queued,
            priority: JobPriority::Normal,
            payload: String::new(),
            attempts: 0,
            progress: 0.0,
            available_at: now,
            last_error: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn default_policy_has_sane_values() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.base_delay, SignedDuration::from_secs(1));
        assert_eq!(policy.multiplier, 2);
        assert_eq!(policy.max_delay, SignedDuration::from_secs(60));
    }

    #[test]
    fn backoff_is_exponential_and_capped() {
        let policy = policy();
        assert_eq!(backoff(1, &policy), SignedDuration::from_secs(1));
        assert_eq!(backoff(2, &policy), SignedDuration::from_secs(2));
        assert_eq!(backoff(3, &policy), SignedDuration::from_secs(4));
        assert_eq!(backoff(100, &policy), SignedDuration::from_secs(60));
    }

    #[test]
    fn success_marks_succeeded() {
        let now = Timestamp::now();
        let out = apply_outcome(job(now), Ok(()), now, &policy());
        assert_eq!(out.status, JobStatus::Succeeded);
        assert_eq!(out.progress, 1.0);
        assert_eq!(out.attempts, 1);
        assert_eq!(out.last_error, None);
    }

    #[test]
    fn retryable_under_max_requeues_with_backoff() {
        let now = Timestamp::now();
        let out = apply_outcome(
            job(now),
            Err(JobError::Retryable("boom".into())),
            now,
            &policy(),
        );
        assert_eq!(out.status, JobStatus::Queued);
        assert_eq!(out.attempts, 1);
        assert_eq!(
            out.available_at,
            now.saturating_add(SignedDuration::from_secs(1)).unwrap()
        );
        assert_eq!(
            out.last_error.as_deref(),
            Some("retryable job failure: boom")
        );
    }

    #[test]
    fn retryable_at_max_fails() {
        let now = Timestamp::now();
        let mut at_last = job(now);
        at_last.attempts = 2;
        let out = apply_outcome(
            at_last,
            Err(JobError::Retryable("boom".into())),
            now,
            &policy(),
        );
        assert_eq!(out.status, JobStatus::Failed);
        assert_eq!(out.attempts, 3);
    }

    #[test]
    fn permanent_fails_immediately() {
        let now = Timestamp::now();
        let out = apply_outcome(
            job(now),
            Err(JobError::Permanent("nope".into())),
            now,
            &policy(),
        );
        assert_eq!(out.status, JobStatus::Failed);
        assert_eq!(out.attempts, 1);
        assert_eq!(
            out.last_error.as_deref(),
            Some("permanent job failure: nope")
        );
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    prop_compose! {
        fn policies()(
            base_ms in 0i64..=100_000,
            max_ms in 0i64..=1_000_000,
            multiplier in 1u32..=10,
        ) -> RetryPolicy {
            RetryPolicy {
                max_attempts: 3,
                base_delay: SignedDuration::from_millis(base_ms),
                multiplier,
                max_delay: SignedDuration::from_millis(max_ms),
            }
        }
    }

    proptest! {
        #[test]
        fn never_exceeds_max_delay(policy in policies(), attempts in 0u32..=64) {
            prop_assert!(backoff(attempts, &policy) <= policy.max_delay);
        }

        #[test]
        fn monotonic_non_decreasing_in_attempts(policy in policies(), a in 0u32..=32, b in 0u32..=32) {
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            prop_assert!(backoff(lo, &policy) <= backoff(hi, &policy));
        }

        #[test]
        fn zero_and_one_attempt_equal_capped_base(policy in policies()) {
            let capped_base = SignedDuration::from_millis(
                policy.base_delay.as_millis().min(policy.max_delay.as_millis()) as i64,
            );
            prop_assert_eq!(backoff(0, &policy), backoff(1, &policy));
            prop_assert_eq!(backoff(0, &policy), capped_base);
        }

        #[test]
        fn no_overflow_at_max_attempts(policy in policies()) {
            prop_assert!(backoff(u32::MAX, &policy) <= policy.max_delay);
        }
    }
}

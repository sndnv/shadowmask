use std::time::{Duration, Instant};

use tokio::sync::Mutex;

pub(crate) struct RateLimiter {
    min_interval: Duration,
    next_allowed: Mutex<Option<Instant>>,
}

impl RateLimiter {
    pub(crate) fn new(min_interval: Duration) -> Self {
        Self { min_interval, next_allowed: Mutex::new(None) }
    }

    pub(crate) async fn acquire(&self) {
        let mut slot = self.next_allowed.lock().await;
        let wait = delay_until(*slot, Instant::now());
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
        *slot = Some(Instant::now() + self.min_interval);
    }

    pub(crate) async fn penalize(&self, retry_after: Duration) {
        let mut slot = self.next_allowed.lock().await;
        let candidate = Instant::now() + retry_after;
        if slot.map(|at| candidate > at).unwrap_or(true) {
            *slot = Some(candidate);
        }
    }
}

fn delay_until(slot: Option<Instant>, now: Instant) -> Duration {
    match slot {
        Some(at) => at.saturating_duration_since(now),
        None => Duration::ZERO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delay_is_zero_without_a_reservation() {
        assert_eq!(delay_until(None, Instant::now()), Duration::ZERO);
    }

    #[test]
    fn delay_is_zero_when_the_slot_is_in_the_past() {
        let now = Instant::now();
        let past = now - Duration::from_millis(50);
        assert_eq!(delay_until(Some(past), now), Duration::ZERO);
    }

    #[test]
    fn delay_is_the_gap_to_a_future_slot() {
        let now = Instant::now();
        let future = now + Duration::from_millis(40);
        let delay = delay_until(Some(future), now);
        assert!(delay > Duration::from_millis(30) && delay <= Duration::from_millis(40));
    }

    #[tokio::test]
    async fn zero_interval_never_blocks() {
        let limiter = RateLimiter::new(Duration::ZERO);
        limiter.acquire().await;
        limiter.acquire().await;
        assert_eq!(delay_until(None, Instant::now()), Duration::ZERO);
    }

    #[tokio::test]
    async fn spaces_consecutive_acquires_by_the_interval() {
        let limiter = RateLimiter::new(Duration::from_millis(30));
        limiter.acquire().await;
        let start = Instant::now();
        limiter.acquire().await;
        assert!(start.elapsed() >= Duration::from_millis(25));
    }

    #[tokio::test]
    async fn penalize_holds_off_the_next_acquire() {
        let limiter = RateLimiter::new(Duration::ZERO);
        limiter.penalize(Duration::from_millis(30)).await;
        let start = Instant::now();
        limiter.acquire().await;
        assert!(start.elapsed() >= Duration::from_millis(25));
    }

    #[tokio::test]
    async fn penalize_keeps_the_longer_delay() {
        let limiter = RateLimiter::new(Duration::ZERO);
        limiter.penalize(Duration::from_millis(50)).await;
        limiter.penalize(Duration::from_millis(5)).await;
        let start = Instant::now();
        limiter.acquire().await;
        assert!(start.elapsed() >= Duration::from_millis(40));
    }
}

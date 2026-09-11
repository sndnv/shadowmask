use jiff::{SignedDuration, Timestamp};

pub struct Debouncer {
    window: SignedDuration,
    pending: bool,
    deadline: Option<Timestamp>,
}

impl Debouncer {
    pub fn new(window: SignedDuration) -> Self {
        Self { window, pending: false, deadline: None }
    }

    pub fn offer(&mut self, now: Timestamp) {
        self.pending = true;
        self.deadline = now.checked_add(self.window).ok();
    }

    pub fn take_ready(&mut self, now: Timestamp) -> bool {
        match self.deadline {
            Some(deadline) if self.pending && now >= deadline => {
                self.pending = false;
                self.deadline = None;
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(text: &str) -> Timestamp {
        text.parse().unwrap()
    }

    #[test]
    fn fires_once_after_quiet_window() {
        let mut debouncer = Debouncer::new(SignedDuration::from_secs(10));
        debouncer.offer(ts("2026-01-01T00:00:00Z"));
        assert!(!debouncer.take_ready(ts("2026-01-01T00:00:05Z")));
        assert!(debouncer.take_ready(ts("2026-01-01T00:00:10Z")));
        assert!(!debouncer.take_ready(ts("2026-01-01T00:00:20Z")));
    }

    #[test]
    fn a_burst_coalesces_into_one_fire() {
        let mut debouncer = Debouncer::new(SignedDuration::from_secs(10));
        debouncer.offer(ts("2026-01-01T00:00:00Z"));
        debouncer.offer(ts("2026-01-01T00:00:05Z"));
        assert!(!debouncer.take_ready(ts("2026-01-01T00:00:12Z")));
        assert!(debouncer.take_ready(ts("2026-01-01T00:00:15Z")));
    }

    #[test]
    fn take_ready_without_changes_is_false() {
        let mut debouncer = Debouncer::new(SignedDuration::from_secs(10));
        assert!(!debouncer.take_ready(ts("2026-01-01T00:00:00Z")));
    }
}

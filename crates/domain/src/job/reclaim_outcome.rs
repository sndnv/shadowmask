pub const RECLAIM_DEAD_LETTER_ERROR: &str =
    "worker exited while the job was running; retry limit reached";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReclaimOutcome {
    pub requeued: usize,
    pub dead_lettered: usize,
}

impl ReclaimOutcome {
    pub fn total(&self) -> usize {
        self.requeued + self.dead_lettered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_counts_both_outcomes() {
        let outcome = ReclaimOutcome {
            requeued: 2,
            dead_lettered: 3,
        };
        assert_eq!(outcome.total(), 5);
        assert_eq!(ReclaimOutcome::default().total(), 0);
    }
}

pub const COMPLETION_PERCENT: u32 = 90;
pub const START_PERCENT: u32 = 5;
pub const START_FLOOR_MS: u64 = 60_000;

pub fn is_complete(position_ms: u64, duration_ms: u64) -> bool {
    duration_ms > 0
        && u128::from(position_ms) * 100 >= u128::from(duration_ms) * u128::from(COMPLETION_PERCENT)
}

pub fn is_started(position_ms: u64, duration_ms: u64) -> bool {
    if position_ms == 0 {
        return false;
    }
    if duration_ms == 0 {
        return true;
    }
    let by_percent = u128::from(duration_ms) * u128::from(START_PERCENT) / 100;
    u128::from(position_ms) >= by_percent.min(u128::from(START_FLOOR_MS))
}

#[cfg(test)]
mod tests {
    use super::{is_complete, is_started};

    #[test]
    fn completes_at_or_past_the_threshold() {
        assert!(!is_complete(8_999, 10_000));
        assert!(is_complete(9_000, 10_000));
        assert!(is_complete(10_000, 10_000));
        assert!(is_complete(20_000, 10_000));
    }

    #[test]
    fn an_unknown_duration_never_completes() {
        assert!(!is_complete(0, 0));
        assert!(!is_complete(5_000, 0));
    }

    #[test]
    fn a_long_title_starts_at_the_absolute_floor() {
        let two_hours = 7_200_000;
        assert!(!is_started(59_999, two_hours));
        assert!(is_started(60_000, two_hours));
    }

    #[test]
    fn a_short_title_starts_at_the_percentage() {
        let three_minutes = 180_000;
        assert!(!is_started(8_999, three_minutes));
        assert!(is_started(9_000, three_minutes));
    }

    #[test]
    fn the_smaller_of_the_two_gates_wins() {
        let ten_minutes = 600_000;
        assert!(is_started(30_000, ten_minutes));
        assert!(!is_started(29_999, ten_minutes));
    }

    #[test]
    fn a_position_of_zero_is_never_started() {
        assert!(!is_started(0, 7_200_000));
        assert!(!is_started(0, 0));
    }

    #[test]
    fn an_unknown_duration_falls_back_to_any_progress() {
        assert!(is_started(1, 0));
        assert!(is_started(5_000, 0));
    }
}

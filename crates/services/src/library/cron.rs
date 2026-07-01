use std::str::FromStr;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use jiff_cron::Schedule;

pub fn next_fire_after(expression: &str, after: Timestamp, tz: TimeZone) -> Option<Timestamp> {
    let schedule = Schedule::from_str(expression).ok()?;
    let next = schedule.after(after.to_zoned(tz)).next()?;
    Some(next.timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(text: &str) -> Timestamp {
        text.parse().unwrap()
    }

    #[test]
    fn computes_next_hourly_fire() {
        let next = next_fire_after("0 0 * * * * *", ts("2026-01-01T00:00:00Z"), TimeZone::UTC);
        assert_eq!(next, Some(ts("2026-01-01T01:00:00Z")));
    }

    #[test]
    fn computes_next_daily_fire() {
        let next = next_fire_after("0 0 3 * * * *", ts("2026-01-01T00:00:00Z"), TimeZone::UTC);
        assert_eq!(next, Some(ts("2026-01-01T03:00:00Z")));
    }

    #[test]
    fn invalid_expression_is_none() {
        assert_eq!(
            next_fire_after("not a cron", ts("2026-01-01T00:00:00Z"), TimeZone::UTC),
            None
        );
    }
}

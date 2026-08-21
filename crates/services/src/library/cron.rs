use std::str::FromStr;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use jiff_cron::Schedule;

pub fn next_fire_after(expression: &str, after: Timestamp, tz: TimeZone) -> Option<Timestamp> {
    let schedule = Schedule::from_str(expression).ok()?;
    let next = schedule.after(after.to_zoned(tz)).next()?;
    Some(next.timestamp())
}

pub fn next_daily_fire(at: &str, after: Timestamp, tz: TimeZone) -> Option<Timestamp> {
    let time: jiff::civil::Time = at.parse().ok()?;
    next_fire_after(
        &format!("0 {} {} * * * *", time.minute(), time.hour()),
        after,
        tz,
    )
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

    #[test]
    fn daily_fire_is_the_next_occurrence_of_the_clock_time() {
        assert_eq!(
            next_daily_fire("04:00", ts("2026-01-01T00:00:00Z"), TimeZone::UTC),
            Some(ts("2026-01-01T04:00:00Z"))
        );
        assert_eq!(
            next_daily_fire("04:00", ts("2026-01-01T06:00:00Z"), TimeZone::UTC),
            Some(ts("2026-01-02T04:00:00Z"))
        );
        assert_eq!(
            next_daily_fire("23:45", ts("2026-01-01T00:00:00Z"), TimeZone::UTC),
            Some(ts("2026-01-01T23:45:00Z"))
        );
    }

    #[test]
    fn daily_fire_rejects_anything_that_is_not_a_clock_time() {
        for value in ["", "off", "4pm", "25:00", "04:61"] {
            assert_eq!(
                next_daily_fire(value, ts("2026-01-01T00:00:00Z"), TimeZone::UTC),
                None,
                "{value} should not parse"
            );
        }
    }
}

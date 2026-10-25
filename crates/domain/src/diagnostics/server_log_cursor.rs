use std::fmt;

use jiff::civil::DateTime;

const HOUR_FORMAT: &str = "%Y-%m-%d-%H";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerLogCursor {
    pub hour: DateTime,
    pub entry: u64,
}

impl ServerLogCursor {
    pub fn parse(text: &str) -> Option<Self> {
        let (hour, entry) = text.split_once(':')?;
        Some(Self { hour: Self::parse_hour(hour)?, entry: entry.parse().ok()? })
    }

    pub fn parse_hour(text: &str) -> Option<DateTime> {
        DateTime::strptime(HOUR_FORMAT, text).ok()
    }
}

impl fmt::Display for ServerLogCursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.hour.strftime(HOUR_FORMAT), self.entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    #[test]
    fn a_cursor_reads_back_from_its_text() {
        let cursor = ServerLogCursor { hour: date(2026, 10, 24).at(9, 0, 0, 0), entry: 1234 };
        assert_eq!(cursor.to_string(), "2026-10-24-09:1234");
        assert_eq!(ServerLogCursor::parse(&cursor.to_string()), Some(cursor));
    }

    #[test]
    fn a_malformed_cursor_does_not_parse() {
        for text in [
            "",
            "2026-10-24-09",
            "2026-10-24-09:",
            "2026-10-24-09:-1",
            "2026-10-24-09:x",
            "2026-13-24-09:1",
            "2026-10-24-24:1",
            "../../etc/passwd:1",
            "2026-10-24:1",
        ] {
            assert_eq!(ServerLogCursor::parse(text), None, "{text}");
        }
    }

    #[test]
    fn an_hour_is_the_files_own_suffix() {
        assert_eq!(
            ServerLogCursor::parse_hour("2026-10-24-13"),
            Some(date(2026, 10, 24).at(13, 0, 0, 0))
        );
        assert_eq!(ServerLogCursor::parse_hour("2026-10-24"), None);
    }
}

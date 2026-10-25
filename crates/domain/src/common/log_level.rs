#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    pub fn parse(token: &str) -> Option<Self> {
        [LogLevel::Debug, LogLevel::Info, LogLevel::Warn, LogLevel::Error]
            .into_iter()
            .find(|level| token.eq_ignore_ascii_case(level.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_round_trips_and_orders() {
        for level in [LogLevel::Debug, LogLevel::Info, LogLevel::Warn, LogLevel::Error] {
            assert_eq!(LogLevel::parse(level.as_str()), Some(level));
        }
        assert_eq!(LogLevel::parse("info"), Some(LogLevel::Info));
        assert_eq!(LogLevel::parse("verbose"), None);
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
    }
}

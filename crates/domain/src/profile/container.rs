#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Container {
    Mp4,
    Mkv,
    Ts,
    Hls,
}

impl Container {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "mp4" => Some(Container::Mp4),
            "mkv" | "webm" => Some(Container::Mkv),
            "ts" => Some(Container::Ts),
            "hls" => Some(Container::Hls),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_containers_and_rejects_unknown() {
        assert_eq!(Container::parse("mp4"), Some(Container::Mp4));
        assert_eq!(Container::parse("mkv"), Some(Container::Mkv));
        assert_eq!(Container::parse("webm"), Some(Container::Mkv));
        assert_eq!(Container::parse("ts"), Some(Container::Ts));
        assert_eq!(Container::parse("hls"), Some(Container::Hls));
        assert_eq!(Container::parse("flv"), None);
    }
}

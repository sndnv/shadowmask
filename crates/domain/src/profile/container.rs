#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Container {
    Mp4,
    Mkv,
    Ts,
    Hls,
    Avi,
    Mov,
    M4v,
    Wmv,
    Flv,
    Mpg,
    M2ts,
}

impl Container {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "mp4" => Some(Container::Mp4),
            "mkv" | "webm" => Some(Container::Mkv),
            "ts" => Some(Container::Ts),
            "hls" => Some(Container::Hls),
            "avi" => Some(Container::Avi),
            "mov" => Some(Container::Mov),
            "m4v" => Some(Container::M4v),
            "wmv" => Some(Container::Wmv),
            "flv" => Some(Container::Flv),
            "mpg" | "mpeg" => Some(Container::Mpg),
            "m2ts" => Some(Container::M2ts),
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
        assert_eq!(Container::parse("avi"), Some(Container::Avi));
        assert_eq!(Container::parse("mov"), Some(Container::Mov));
        assert_eq!(Container::parse("m4v"), Some(Container::M4v));
        assert_eq!(Container::parse("wmv"), Some(Container::Wmv));
        assert_eq!(Container::parse("flv"), Some(Container::Flv));
        assert_eq!(Container::parse("mpg"), Some(Container::Mpg));
        assert_eq!(Container::parse("mpeg"), Some(Container::Mpg));
        assert_eq!(Container::parse("m2ts"), Some(Container::M2ts));
        assert_eq!(Container::parse("iso"), None);
        assert_eq!(Container::parse("MP4"), None);
    }
}

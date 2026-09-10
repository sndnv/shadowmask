#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HdrFormat {
    Hdr10,
    Hdr10Plus,
    DolbyVision,
    Hlg,
}

impl HdrFormat {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "hdr10" => Some(HdrFormat::Hdr10),
            "hdr10plus" | "hdr10_plus" => Some(HdrFormat::Hdr10Plus),
            "dolby_vision" => Some(HdrFormat::DolbyVision),
            "hlg" => Some(HdrFormat::Hlg),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_formats_and_rejects_unknown() {
        assert_eq!(HdrFormat::parse("hdr10"), Some(HdrFormat::Hdr10));
        // The profile data files spell it one way and the wire the other, and
        // both reach this function.
        assert_eq!(HdrFormat::parse("hdr10plus"), Some(HdrFormat::Hdr10Plus));
        assert_eq!(HdrFormat::parse("hdr10_plus"), Some(HdrFormat::Hdr10Plus));
        assert_eq!(
            HdrFormat::parse("dolby_vision"),
            Some(HdrFormat::DolbyVision)
        );
        assert_eq!(HdrFormat::parse("hlg"), Some(HdrFormat::Hlg));
        assert_eq!(HdrFormat::parse("hdr11"), None);
    }
}

#[derive(Debug, Clone)]
pub struct VideoTrack {
    pub index: u32,
    pub codec: String,
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,
    pub hdr: Option<HdrFormat>,
    pub frame_rate: f32,
    pub bitrate: Option<u64>,
}

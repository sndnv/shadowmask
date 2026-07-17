#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubtitleFormat {
    Srt,
    Ass,
    Vtt,
    Pgs,
    VobSub,
}

static EXT_TABLE: &[(&str, SubtitleFormat)] = &[
    ("srt", SubtitleFormat::Srt),
    ("ass", SubtitleFormat::Ass),
    ("ssa", SubtitleFormat::Ass),
    ("vtt", SubtitleFormat::Vtt),
    ("sub", SubtitleFormat::VobSub),
];

impl SubtitleFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        let ext = ext.trim_start_matches('.').to_ascii_lowercase();
        EXT_TABLE
            .iter()
            .find(|entry| entry.0 == ext.as_str())
            .map(|entry| entry.1)
    }

    pub fn extension(self) -> &'static str {
        match self {
            SubtitleFormat::Srt => "srt",
            SubtitleFormat::Ass => "ass",
            SubtitleFormat::Vtt => "vtt",
            SubtitleFormat::Pgs => "sup",
            SubtitleFormat::VobSub => "sub",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_extensions_map_to_a_format() {
        assert_eq!(
            SubtitleFormat::from_extension("srt"),
            Some(SubtitleFormat::Srt)
        );
        assert_eq!(
            SubtitleFormat::from_extension(".VTT"),
            Some(SubtitleFormat::Vtt)
        );
        assert_eq!(
            SubtitleFormat::from_extension("ssa"),
            Some(SubtitleFormat::Ass)
        );
        assert_eq!(
            SubtitleFormat::from_extension("sub"),
            Some(SubtitleFormat::VobSub)
        );
    }

    #[test]
    fn unknown_extension_has_no_format() {
        assert_eq!(SubtitleFormat::from_extension("mkv"), None);
    }

    #[test]
    fn extension_names_each_format() {
        assert_eq!(SubtitleFormat::Srt.extension(), "srt");
        assert_eq!(SubtitleFormat::Ass.extension(), "ass");
        assert_eq!(SubtitleFormat::Vtt.extension(), "vtt");
        assert_eq!(SubtitleFormat::Pgs.extension(), "sup");
        assert_eq!(SubtitleFormat::VobSub.extension(), "sub");
    }
}

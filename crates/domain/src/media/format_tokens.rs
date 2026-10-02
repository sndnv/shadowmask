pub struct FormatToken {
    pub text: &'static str,
    pub truncates_a_title: bool,
}

const fn noise(text: &'static str) -> FormatToken {
    FormatToken { text, truncates_a_title: true }
}

const fn tail(text: &'static str) -> FormatToken {
    FormatToken { text, truncates_a_title: false }
}

pub const FORMAT_TOKENS: &[FormatToken] = &[
    noise("480p"),
    noise("576p"),
    noise("720p"),
    noise("1080p"),
    noise("2160p"),
    noise("4k"),
    noise("x264"),
    noise("x265"),
    noise("h264"),
    noise("h265"),
    noise("h.264"),
    noise("h.265"),
    noise("hevc"),
    noise("xvid"),
    tail("avc"),
    tail("divx"),
    tail("av1"),
    noise("bluray"),
    noise("blu-ray"),
    noise("hdtv"),
    noise("remux"),
    noise("bdrip"),
    noise("brrip"),
    noise("webrip"),
    noise("web-dl"),
    noise("webdl"),
    noise("dvdrip"),
    noise("hdrip"),
    noise("tvrip"),
    tail("web"),
    tail("dl"),
    tail("ray"),
    tail("rip"),
    tail("hd"),
    tail("uhd"),
    tail("sdr"),
    tail("hdr"),
    noise("dts"),
    noise("aac"),
    noise("ac3"),
    tail("eac3"),
    tail("ma"),
    tail("es"),
    tail("dd"),
    tail("ddp"),
    tail("atmos"),
    tail("truehd"),
    tail("bit"),
    tail("8bit"),
    tail("10bit"),
    tail("12bit"),
    tail("extended"),
    tail("remastered"),
    tail("uncut"),
    tail("unrated"),
    tail("imax"),
    tail("3d"),
    tail("dc"),
    tail("subs"),
    tail("multi"),
    tail("dual"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_token_is_listed_once() {
        let mut seen: Vec<&str> = FORMAT_TOKENS.iter().map(|token| token.text).collect();
        let total = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), total);
    }

    #[test]
    fn a_noise_token_cuts_a_title_and_a_tail_token_does_not() {
        assert!(noise("x264").truncates_a_title);
        assert!(!tail("web").truncates_a_title);
    }

    #[test]
    fn a_hyphenated_token_is_only_ever_read_by_the_parser() {
        for token in FORMAT_TOKENS.iter().filter(|token| token.text.contains('-')) {
            assert!(token.truncates_a_title, "[{}]", token.text);
        }
    }
}

use std::sync::LazyLock;

use regex::{Captures, Regex};

static TIMESTAMP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d{2}):(\d{2}):(\d{2})([.,])(\d{3})").unwrap());

pub fn shift(content: &str, offset_ms: i64) -> String {
    TIMESTAMP
        .replace_all(content, |caps: &Captures| {
            let hours: i64 = caps[1].parse().unwrap();
            let minutes: i64 = caps[2].parse().unwrap();
            let seconds: i64 = caps[3].parse().unwrap();
            let separator = &caps[4];
            let millis: i64 = caps[5].parse().unwrap();
            let total = hours * 3_600_000 + minutes * 60_000 + seconds * 1_000 + millis;
            let shifted = (total + offset_ms).max(0);
            format!(
                "{:02}:{:02}:{:02}{}{:03}",
                shifted / 3_600_000,
                shifted / 60_000 % 60,
                shifted / 1_000 % 60,
                separator,
                shifted % 1_000,
            )
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shifts_srt_cue_forward() {
        let srt = "1\n00:00:01,000 --> 00:00:04,000\nHello\n";
        let out = shift(srt, 2_500);
        assert!(out.contains("00:00:03,500 --> 00:00:06,500"));
        assert!(out.contains("Hello"));
    }

    #[test]
    fn shifts_vtt_cue_and_preserves_dot_separator() {
        let vtt = "WEBVTT\n\n00:01:00.000 --> 00:01:02.500\nHi\n";
        let out = shift(vtt, 1_000);
        assert!(out.contains("00:01:01.000 --> 00:01:03.500"));
    }

    #[test]
    fn negative_offset_clamps_at_zero() {
        let srt = "00:00:01,000 --> 00:00:02,000\n";
        let out = shift(srt, -5_000);
        assert!(out.contains("00:00:00,000 --> 00:00:00,000"));
    }

    #[test]
    fn lines_without_timestamps_pass_through() {
        let text = "WEBVTT\n\nNOTE just a comment\n";
        assert_eq!(shift(text, 10_000), text);
    }
}

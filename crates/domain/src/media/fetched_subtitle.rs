use std::collections::HashSet;

use crate::media::SubtitleFormat;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedSubtitle {
    pub content: String,
    pub format: SubtitleFormat,
}

impl FetchedSubtitle {
    pub fn has_text(&self) -> bool {
        subtitle_has_text(&self.content)
    }
}

pub fn subtitle_has_text(content: &str) -> bool {
    content.lines().any(|line| !is_structural(line))
}

pub fn is_hallucinated_text(text: &str) -> bool {
    let normalized = normalize(text);
    if normalized.is_empty() {
        return false;
    }
    if HALLUCINATION_SUBSTRINGS
        .iter()
        .any(|needle| normalized.contains(needle))
    {
        return true;
    }
    is_repetitive(&normalized.split(' ').collect::<Vec<_>>()) || is_low_diversity(&normalized)
}

const HALLUCINATION_SUBSTRINGS: &[&str] = &[
    "subtitles by",
    "subtitle by",
    "subs by",
    "subbed by",
    "subtitled by",
    "subtitles provided by",
    "subtitles created by",
    "captions by",
    "captioning by",
    "corrected by",
    "synced by",
    "sync and corrections by",
    "translation by",
    "translated by",
    "amara org",
    "thanks for watching",
    "thank you for watching",
    "please subscribe",
    "like and subscribe",
    "dont forget to subscribe",
    "see you next time",
    "see you in the next video",
];

fn is_repetitive(words: &[&str]) -> bool {
    if words.len() < 6 {
        return false;
    }
    let distinct: HashSet<&&str> = words.iter().collect();
    distinct.len() <= words.len() / 4
}

fn is_low_diversity(normalized: &str) -> bool {
    let chars: Vec<char> = normalized.chars().filter(|c| *c != ' ').collect();
    if chars.len() < 16 {
        return false;
    }
    let distinct: HashSet<char> = chars.iter().copied().collect();
    distinct.len() <= 2
}

fn normalize(text: &str) -> String {
    let mut out = String::new();
    let mut pending_space = false;
    for c in text.chars() {
        if c.is_alphanumeric() {
            if pending_space && !out.is_empty() {
                out.push(' ');
            }
            pending_space = false;
            out.extend(c.to_lowercase());
        } else {
            pending_space = true;
        }
    }
    out
}

fn is_structural(line: &str) -> bool {
    let line = line.trim();
    line.is_empty()
        || line == "WEBVTT"
        || line.starts_with("WEBVTT ")
        || line.starts_with("NOTE")
        || line.contains("-->")
        || line.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vtt(content: &str) -> FetchedSubtitle {
        FetchedSubtitle {
            content: content.into(),
            format: SubtitleFormat::Vtt,
        }
    }

    #[test]
    fn header_only_vtt_has_no_text() {
        assert!(!vtt("WEBVTT\n").has_text());
    }

    #[test]
    fn cue_without_text_has_no_text() {
        assert!(!vtt("WEBVTT\n\n00:00:00.000 --> 00:00:01.000\n").has_text());
    }

    #[test]
    fn cue_with_text_has_text() {
        assert!(vtt("WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nhello\n").has_text());
    }

    #[test]
    fn srt_index_lines_are_not_text() {
        assert!(!subtitle_has_text("1\n00:00:00,000 --> 00:00:01,000\n\n"));
        assert!(subtitle_has_text("1\n00:00:00,000 --> 00:00:01,000\nhi\n"));
    }

    #[test]
    fn vtt_header_with_metadata_is_not_text() {
        assert!(!vtt("WEBVTT - Kind: captions\n").has_text());
    }

    #[test]
    fn blank_content_has_no_text() {
        assert!(!subtitle_has_text("   \n\n"));
    }

    #[test]
    fn note_blocks_are_not_text() {
        assert!(!subtitle_has_text("WEBVTT\n\nNOTE recorded by whisper\n"));
    }

    #[test]
    fn credit_lines_are_hallucinations() {
        assert!(is_hallucinated_text("Subtitles by TeamWhatever"));
        assert!(is_hallucinated_text("Subs by the Amara.org community"));
        assert!(is_hallucinated_text("Thanks for watching!"));
        assert!(is_hallucinated_text("Please subscribe and like"));
    }

    #[test]
    fn real_dialogue_is_not_a_hallucination() {
        assert!(!is_hallucinated_text("I will be there by nine tonight"));
        assert!(!is_hallucinated_text("Hello there, how are you?"));
    }

    #[test]
    fn single_word_repetition_is_a_hallucination() {
        assert!(is_hallucinated_text("the the the the the the the the"));
        assert!(is_hallucinated_text(
            "thank you thank you thank you thank you thank you thank you thank you"
        ));
    }

    #[test]
    fn short_repeats_are_kept() {
        assert!(!is_hallucinated_text("no no no"));
        assert!(!is_hallucinated_text("run run"));
    }

    #[test]
    fn empty_text_is_not_a_hallucination() {
        assert!(!is_hallucinated_text("   "));
    }

    #[test]
    fn single_token_character_loop_is_a_hallucination() {
        let blob = "\u{06d5}".repeat(200);
        assert!(is_hallucinated_text(&blob));
        assert!(is_hallucinated_text("aaaaaaaaaaaaaaaaaaaa"));
    }

    #[test]
    fn varied_long_text_is_not_low_diversity() {
        assert!(!is_hallucinated_text(
            "the quick brown fox jumps over the lazy dog near the river"
        ));
    }
}

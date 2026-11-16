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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HallucinationFilter {
    phrases: Vec<String>,
}

impl HallucinationFilter {
    pub fn new<S: AsRef<str>>(phrases: &[S]) -> Self {
        let phrases = phrases
            .iter()
            .map(|phrase| normalize(phrase.as_ref()))
            .filter(|phrase| !phrase.is_empty())
            .collect();
        Self { phrases }
    }

    pub fn phrase_count(&self) -> usize {
        self.phrases.len()
    }

    pub fn is_hallucinated(&self, text: &str) -> bool {
        let normalized = normalize(text);
        if normalized.is_empty() {
            return false;
        }
        if self.phrases.iter().any(|phrase| normalized.contains(phrase.as_str())) {
            return true;
        }
        is_repetitive(&normalized.split(' ').collect::<Vec<_>>()) || is_low_diversity(&normalized)
    }
}

impl Default for HallucinationFilter {
    fn default() -> Self {
        Self::new(DEFAULT_HALLUCINATION_PHRASES)
    }
}

pub const DEFAULT_HALLUCINATION_PHRASES: &[&str] = &[
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
        if matches!(c, '\'' | '\u{2019}') {
            continue;
        }
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
        FetchedSubtitle { content: content.into(), format: SubtitleFormat::Vtt }
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

    fn rejects(text: &str) -> bool {
        HallucinationFilter::default().is_hallucinated(text)
    }

    #[test]
    fn credit_lines_are_hallucinations() {
        assert!(rejects("Subtitles by TeamWhatever"));
        assert!(rejects("Subs by the Amara.org community"));
        assert!(rejects("Thanks for watching!"));
        assert!(rejects("Please subscribe and like"));
    }

    #[test]
    fn real_dialogue_is_not_a_hallucination() {
        assert!(!rejects("I will be there by nine tonight"));
        assert!(!rejects("Hello there, how are you?"));
    }

    #[test]
    fn single_word_repetition_is_a_hallucination() {
        assert!(rejects("the the the the the the the the"));
        assert!(rejects("thank you thank you thank you thank you thank you thank you thank you"));
    }

    #[test]
    fn short_repeats_are_kept() {
        assert!(!rejects("no no no"));
        assert!(!rejects("run run"));
    }

    #[test]
    fn empty_text_is_not_a_hallucination() {
        assert!(!rejects("   "));
    }

    #[test]
    fn single_token_character_loop_is_a_hallucination() {
        let blob = "\u{06d5}".repeat(200);
        assert!(rejects(&blob));
        assert!(rejects("aaaaaaaaaaaaaaaaaaaa"));
    }

    #[test]
    fn varied_long_text_is_not_low_diversity() {
        assert!(!rejects("the quick brown fox jumps over the lazy dog near the river"));
    }

    #[test]
    fn the_default_filter_holds_the_english_phrases() {
        assert_eq!(
            HallucinationFilter::default(),
            HallucinationFilter::new(DEFAULT_HALLUCINATION_PHRASES)
        );
        for phrase in DEFAULT_HALLUCINATION_PHRASES {
            assert!(rejects(&format!("Cue: {phrase}!")), "{phrase}");
        }
    }

    #[test]
    fn configured_phrases_replace_the_defaults() {
        let filter = HallucinationFilter::new(&["字幕志愿者", "subtitle volunteer"]);

        assert!(filter.is_hallucinated("字幕志愿者 李宗盛"));
        assert!(filter.is_hallucinated(") Subtitle volunteer: Lee Jong-seong"));
        assert!(!filter.is_hallucinated("Thanks for watching!"));
        assert!(!filter.is_hallucinated("真的是美啊"));
    }

    #[test]
    fn phrases_are_written_the_way_the_text_is_read() {
        let filter = HallucinationFilter::new(&["  Subtitles,   BY! "]);

        assert!(filter.is_hallucinated("subtitles by someone"));
        assert!(filter.is_hallucinated("SUBTITLES - by - someone"));
        assert!(!filter.is_hallucinated("subtitle by someone"));
    }

    #[test]
    fn an_apostrophe_joins_the_word_it_sits_in() {
        assert!(rejects("Don't forget to subscribe!"));
        assert!(rejects("Don’t forget to subscribe"));
        let filter = HallucinationFilter::new(&["won't stop"]);
        assert!(filter.is_hallucinated("I wont stop"));
        assert!(filter.is_hallucinated("I won’t stop"));
        assert!(!filter.is_hallucinated("I won t stop"));
    }

    #[test]
    fn the_filter_counts_only_the_phrases_it_keeps() {
        assert_eq!(
            HallucinationFilter::default().phrase_count(),
            DEFAULT_HALLUCINATION_PHRASES.len()
        );
        assert_eq!(
            HallucinationFilter::new(&["字幕志愿者", "", "-", " subtitle volunteer "])
                .phrase_count(),
            2
        );
    }

    #[test]
    fn a_blank_phrase_matches_nothing() {
        let filter = HallucinationFilter::new(&["", "   ", "!!", "-"]);

        assert!(!filter.is_hallucinated("I will be there by nine tonight"));
        assert!(!filter.is_hallucinated("真的是美啊"));
    }

    #[test]
    fn the_repetition_checks_stay_whatever_the_phrases() {
        let filter = HallucinationFilter::new::<&str>(&[]);

        assert!(filter.is_hallucinated("the the the the the the the the"));
        assert!(filter.is_hallucinated("aaaaaaaaaaaaaaaaaaaa"));
        assert!(!filter.is_hallucinated("Subtitles by TeamWhatever"));
    }
}

use std::io::Write;

use flate2::Compression;
use flate2::write::ZlibEncoder;

use crate::Segment;
use crate::cue_split::split_cue;

pub const WINDOW_FRAMES: usize = 3_000;
const FRAME_MS: u64 = 10;
const TIMESTAMP_MS: u64 = 20;
const FRAMES_PER_TIMESTAMP: usize = 2;
const LANGUAGE_FRAMES: usize = 24_000;
const PROMPT_TOKENS: usize = 223;
const TEMPERATURES: [f32; 6] = [0.0, 0.2, 0.4, 0.6, 0.8, 1.0];
const COMPRESSION_RATIO_LIMIT: f32 = 2.4;
const LOG_PROB_FLOOR: f32 = -1.0;
const NO_SPEECH_LIMIT: f32 = 0.6;
const PROMPT_RESET_TEMPERATURE: f32 = 0.5;

pub struct Attempt {
    pub tokens: Vec<u32>,
    pub score: f32,
    pub no_speech_prob: f32,
}

pub struct Vocabulary {
    pub start_of_prev: u32,
    pub start_of_transcript: u32,
    pub transcribe: u32,
    pub end_of_text: u32,
    pub timestamp_begin: u32,
    pub multilingual: bool,
}

pub trait WindowDecoder {
    type Error;

    fn load(&mut self, start_frame: usize) -> Result<(), Self::Error>;

    fn detect_language(&mut self) -> Result<u32, Self::Error>;

    fn decode(&mut self, prompt: &[u32], temperature: f32) -> Result<Attempt, Self::Error>;

    fn text(&self, tokens: &[u32]) -> Result<String, Self::Error>;
}

pub fn transcribe<D: WindowDecoder>(
    decoder: &mut D,
    vocabulary: &Vocabulary,
    frames: usize,
    language: Option<u32>,
) -> Result<Vec<Segment>, D::Error> {
    let mut segments = Vec::new();
    let mut kept = Vec::new();
    let mut prompt_start = 0;
    let mut detected = None;
    let mut seek = 0;
    while seek < frames {
        let size = WINDOW_FRAMES.min(frames - seek);
        decoder.load(seek)?;
        let language = match (vocabulary.multilingual, language) {
            (false, _) => None,
            (true, Some(token)) => Some(token),
            (true, None) => Some(detected_language(decoder, seek, &mut detected)?),
        };
        let prompt = prompt(vocabulary, &kept[prompt_start..], language);
        let decoded = decode_with_fallback(decoder, vocabulary, &prompt)?;
        if decoded.is_silence() {
            seek += size;
            continue;
        }
        let (lines, next) =
            split_at_timestamps(&decoded.attempt.tokens, vocabulary.timestamp_begin, seek, size);
        for line in lines {
            let text = decoder.text(&text_tokens(&line.tokens, vocabulary))?;
            let text = text.trim();
            if line.start_ms == line.end_ms || text.is_empty() {
                continue;
            }
            segments.push(Segment {
                start_ms: line.start_ms,
                end_ms: line.end_ms,
                text: text.to_owned(),
            });
            kept.extend(line.tokens);
        }
        if decoded.temperature > PROMPT_RESET_TEMPERATURE {
            prompt_start = kept.len();
        }
        seek = if next > seek { next } else { seek + size };
    }
    Ok(segments.into_iter().flat_map(split_cue).collect())
}

struct Decoded {
    attempt: Attempt,
    avg_log_prob: f32,
    repetitive: bool,
    temperature: f32,
}

impl Decoded {
    fn is_silence(&self) -> bool {
        self.attempt.no_speech_prob > NO_SPEECH_LIMIT && self.avg_log_prob < LOG_PROB_FLOOR
    }

    fn is_accepted(&self) -> bool {
        self.is_silence() || !(self.repetitive || self.avg_log_prob < LOG_PROB_FLOOR)
    }

    fn ranks_above(&self, other: &Decoded) -> bool {
        (!self.repetitive, self.avg_log_prob) > (!other.repetitive, other.avg_log_prob)
    }
}

struct Line {
    start_ms: u64,
    end_ms: u64,
    tokens: Vec<u32>,
}

fn detected_language<D: WindowDecoder>(
    decoder: &mut D,
    seek: usize,
    detected: &mut Option<(usize, u32)>,
) -> Result<u32, D::Error> {
    let block = seek / LANGUAGE_FRAMES;
    match *detected {
        Some((last, token)) if last == block => Ok(token),
        _ => {
            let token = decoder.detect_language()?;
            *detected = Some((block, token));
            Ok(token)
        }
    }
}

fn prompt(vocabulary: &Vocabulary, previous: &[u32], language: Option<u32>) -> Vec<u32> {
    let mut prompt = Vec::new();
    if !previous.is_empty() {
        prompt.push(vocabulary.start_of_prev);
        prompt.extend(&previous[previous.len().saturating_sub(PROMPT_TOKENS)..]);
    }
    prompt.push(vocabulary.start_of_transcript);
    if let Some(language) = language {
        prompt.extend([language, vocabulary.transcribe]);
    }
    prompt
}

fn decode_with_fallback<D: WindowDecoder>(
    decoder: &mut D,
    vocabulary: &Vocabulary,
    prompt: &[u32],
) -> Result<Decoded, D::Error> {
    let first = decode_at(decoder, vocabulary, prompt, TEMPERATURES[0])?;
    if first.is_accepted() {
        return Ok(first);
    }
    let mut best = first;
    for &temperature in &TEMPERATURES[1..] {
        let next = decode_at(decoder, vocabulary, prompt, temperature)?;
        if next.is_accepted() {
            return Ok(next);
        }
        if next.ranks_above(&best) {
            best = next;
        }
    }
    Ok(Decoded { temperature: TEMPERATURES[TEMPERATURES.len() - 1], ..best })
}

fn decode_at<D: WindowDecoder>(
    decoder: &mut D,
    vocabulary: &Vocabulary,
    prompt: &[u32],
    temperature: f32,
) -> Result<Decoded, D::Error> {
    let attempt = decoder.decode(prompt, temperature)?;
    let text = decoder.text(&text_tokens(&attempt.tokens, vocabulary))?;
    Ok(Decoded {
        avg_log_prob: average_log_prob(attempt.score, attempt.tokens.len()),
        repetitive: compression_ratio(text.trim()) > COMPRESSION_RATIO_LIMIT,
        attempt,
        temperature,
    })
}

fn split_at_timestamps(
    tokens: &[u32],
    timestamp_begin: u32,
    seek: usize,
    size: usize,
) -> (Vec<Line>, usize) {
    let is_timestamp = |index: usize| tokens[index] >= timestamp_begin;
    let position = |token: u32| u64::from(token.saturating_sub(timestamp_begin));
    let offset_ms = seek as u64 * FRAME_MS;
    let mut cuts: Vec<usize> =
        (1..tokens.len()).filter(|&index| is_timestamp(index) && is_timestamp(index - 1)).collect();
    if cuts.is_empty() {
        let end_ms = tokens
            .iter()
            .rev()
            .find(|token| **token >= timestamp_begin)
            .map(|token| position(*token))
            .filter(|position| *position > 0)
            .map_or(size as u64 * FRAME_MS, |position| position * TIMESTAMP_MS);
        let line =
            Line { start_ms: offset_ms, end_ms: offset_ms + end_ms, tokens: tokens.to_vec() };
        return (vec![line], seek + size);
    }
    let single_ending =
        tokens.len() >= 2 && !is_timestamp(tokens.len() - 2) && is_timestamp(tokens.len() - 1);
    if single_ending {
        cuts.push(tokens.len());
    }
    let mut lines = Vec::with_capacity(cuts.len());
    let mut start = 0;
    for cut in cuts {
        let slice = &tokens[start..cut];
        lines.push(Line {
            start_ms: offset_ms + position(slice[0]) * TIMESTAMP_MS,
            end_ms: offset_ms + position(slice[slice.len() - 1]) * TIMESTAMP_MS,
            tokens: slice.to_vec(),
        });
        start = cut;
    }
    let next = if single_ending {
        seek + size
    } else {
        seek + position(tokens[start - 1]) as usize * FRAMES_PER_TIMESTAMP
    };
    (lines, next)
}

fn text_tokens(tokens: &[u32], vocabulary: &Vocabulary) -> Vec<u32> {
    tokens.iter().copied().filter(|token| *token < vocabulary.end_of_text).collect()
}

fn compression_ratio(text: &str) -> f32 {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    let compressed = encoder
        .write_all(text.as_bytes())
        .and_then(|()| encoder.finish())
        .map_or(0, |bytes| bytes.len());
    text.len() as f32 / compressed as f32
}

fn average_log_prob(score: f32, tokens: usize) -> f32 {
    let length = tokens as f32;
    score * length / (length + 1.0)
}

pub fn timestamp_token(position: u32) -> String {
    let hundredths = u64::from(position) * TIMESTAMP_MS / 10;
    format!("<|{}.{:02}|>", hundredths / 100, hundredths % 100)
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, VecDeque};

    use super::*;

    const START_OF_PREV: u32 = 1;
    const START_OF_TRANSCRIPT: u32 = 2;
    const TRANSCRIBE: u32 = 3;
    const ZH: u32 = 50;
    const EN: u32 = 51;
    const END_OF_TEXT: u32 = 900;
    const TIMESTAMP_BEGIN: u32 = 1_000;
    const LONG_LINE: &str = " This is the first sentence. This is the second sentence which is a \
        bit longer. And here is a third sentence to push things well past the limit.";

    fn vocabulary() -> Vocabulary {
        Vocabulary {
            start_of_prev: START_OF_PREV,
            start_of_transcript: START_OF_TRANSCRIPT,
            transcribe: TRANSCRIBE,
            end_of_text: END_OF_TEXT,
            timestamp_begin: TIMESTAMP_BEGIN,
            multilingual: true,
        }
    }

    fn at(ms: u32) -> u32 {
        TIMESTAMP_BEGIN + ms / 20
    }

    fn spoken(tokens: Vec<u32>) -> Attempt {
        Attempt { tokens, score: -0.1, no_speech_prob: 0.0 }
    }

    fn unsure(tokens: Vec<u32>, score: f32) -> Attempt {
        Attempt { tokens, score, no_speech_prob: 0.0 }
    }

    fn silence() -> Attempt {
        Attempt { tokens: vec![at(0), 11, at(2_000)], score: -2.0, no_speech_prob: 0.9 }
    }

    fn cue(start_ms: u64, end_ms: u64, text: &str) -> Segment {
        Segment { start_ms, end_ms, text: text.to_owned() }
    }

    #[derive(Default)]
    struct Script {
        attempts: VecDeque<Attempt>,
        languages: VecDeque<u32>,
        words: HashMap<u32, &'static str>,
        fail_load: bool,
        loads: Vec<usize>,
        detections: Vec<usize>,
        decodes: Vec<(Vec<u32>, f32)>,
    }

    impl Script {
        fn new(attempts: impl IntoIterator<Item = Attempt>) -> Self {
            let words = HashMap::from([
                (10, " hello"),
                (11, " there"),
                (12, " friend"),
                (13, "   "),
                (14, " la la la la la la la la la la la la la la la la la la la la la la la la"),
                (15, LONG_LINE),
            ]);
            Self { attempts: attempts.into_iter().collect(), words, ..Self::default() }
        }

        fn prompts(&self) -> Vec<Vec<u32>> {
            self.decodes.iter().map(|(prompt, _)| prompt.clone()).collect()
        }

        fn temperatures(&self) -> Vec<f32> {
            self.decodes.iter().map(|(_, temperature)| *temperature).collect()
        }
    }

    impl WindowDecoder for Script {
        type Error = String;

        fn load(&mut self, start_frame: usize) -> Result<(), String> {
            if self.fail_load {
                return Err("load failed".to_owned());
            }
            self.loads.push(start_frame);
            Ok(())
        }

        fn detect_language(&mut self) -> Result<u32, String> {
            self.detections.push(self.loads.last().copied().unwrap_or_default());
            self.languages.pop_front().ok_or_else(|| "no language".to_owned())
        }

        fn decode(&mut self, prompt: &[u32], temperature: f32) -> Result<Attempt, String> {
            self.decodes.push((prompt.to_vec(), temperature));
            self.attempts.pop_front().ok_or_else(|| "out of attempts".to_owned())
        }

        fn text(&self, tokens: &[u32]) -> Result<String, String> {
            Ok(tokens
                .iter()
                .map(|token| match self.words.get(token) {
                    Some(word) => (*word).to_owned(),
                    None => char::from_u32(0x4E00 + token * 7_919 % 20_000)
                        .map(String::from)
                        .unwrap_or_default(),
                })
                .collect())
        }
    }

    fn run(script: &mut Script, frames: usize, language: Option<u32>) -> Vec<Segment> {
        transcribe(script, &vocabulary(), frames, language).unwrap()
    }

    #[test]
    fn a_window_moves_to_the_end_of_its_last_complete_line() {
        let mut script = Script::new([
            spoken(vec![at(0), 10, at(2_000), at(2_000), 11, at(5_000), 12]),
            spoken(vec![at(0), 11, 12, at(3_000)]),
        ]);

        let segments = run(&mut script, 3_200, Some(ZH));

        assert_eq!(script.loads, vec![0, 200]);
        assert_eq!(segments, vec![cue(0, 2_000, "hello"), cue(2_000, 5_000, "there friend")]);
    }

    #[test]
    fn a_single_timestamp_ending_moves_a_full_window() {
        let mut script =
            Script::new([spoken(vec![at(0), 10, at(1_000), at(1_000), 11, at(3_000)]), silence()]);

        let segments = run(&mut script, 4_000, Some(ZH));

        assert_eq!(script.loads, vec![0, 3_000]);
        assert_eq!(segments, vec![cue(0, 1_000, "hello"), cue(1_000, 3_000, "there")]);
    }

    #[test]
    fn output_without_timestamps_spans_the_window() {
        let mut script = Script::new([spoken(vec![10, 11])]);

        let segments = run(&mut script, 1_500, Some(ZH));

        assert_eq!(segments, vec![cue(0, 15_000, "hello there")]);
    }

    #[test]
    fn a_lone_opening_timestamp_spans_the_window() {
        let mut script = Script::new([spoken(vec![at(0), 10])]);

        let segments = run(&mut script, 1_500, Some(ZH));

        assert_eq!(segments, vec![cue(0, 15_000, "hello")]);
    }

    #[test]
    fn the_last_window_spans_only_the_audio_left() {
        let mut script = Script::new([spoken(vec![at(0), 10, at(1_000)]), spoken(vec![at(0), 11])]);

        let segments = run(&mut script, 4_000, Some(ZH));

        assert_eq!(segments, vec![cue(0, 1_000, "hello"), cue(30_000, 40_000, "there")]);
    }

    #[test]
    fn a_window_that_would_not_move_moves_a_full_window() {
        let mut script = Script::new([spoken(vec![at(0), 10, at(0), at(0), 11]), silence()]);

        let segments = run(&mut script, 4_000, Some(ZH));

        assert_eq!(script.loads, vec![0, 3_000]);
        assert!(segments.is_empty());
    }

    #[test]
    fn lines_without_text_or_duration_are_dropped_and_left_out_of_the_prompt() {
        let mut script = Script::new([
            spoken(vec![
                at(0),
                at(1_000),
                at(1_000),
                13,
                at(2_000),
                at(2_000),
                10,
                at(3_000),
                at(3_000),
                11,
            ]),
            silence(),
        ]);

        let segments = run(&mut script, 3_000, Some(ZH));

        assert_eq!(segments, vec![cue(2_000, 3_000, "hello")]);
        assert_eq!(script.loads, vec![0, 300]);
        assert_eq!(
            script.prompts()[1],
            vec![START_OF_PREV, at(2_000), 10, at(3_000), START_OF_TRANSCRIPT, ZH, TRANSCRIBE]
        );
    }

    #[test]
    fn the_previous_text_prompts_the_next_window() {
        let mut script =
            Script::new([spoken(vec![at(0), 10, at(2_000), at(2_000), 11]), silence()]);

        run(&mut script, 3_000, Some(ZH));

        assert_eq!(
            script.prompts(),
            vec![
                vec![START_OF_TRANSCRIPT, ZH, TRANSCRIBE],
                vec![START_OF_PREV, at(0), 10, at(2_000), START_OF_TRANSCRIPT, ZH, TRANSCRIBE],
            ]
        );
    }

    #[test]
    fn the_prompt_keeps_the_last_223_tokens() {
        let line: Vec<u32> =
            std::iter::once(at(0)).chain(100..398).chain(std::iter::once(at(2_000))).collect();
        let mut tokens = line.clone();
        tokens.extend([at(2_000), 11]);
        let mut script = Script::new([spoken(tokens), silence()]);

        run(&mut script, 3_000, Some(ZH));

        let mut expected = vec![START_OF_PREV];
        expected.extend(&line[line.len() - 223..]);
        expected.extend([START_OF_TRANSCRIPT, ZH, TRANSCRIBE]);
        assert_eq!(script.prompts()[1], expected);
    }

    #[test]
    fn a_repetitive_attempt_is_decoded_again_hotter() {
        let mut script =
            Script::new([spoken(vec![at(0), 14, at(2_000)]), spoken(vec![at(0), 10, at(2_000)])]);

        let segments = run(&mut script, 3_000, Some(ZH));

        assert_eq!(script.temperatures(), vec![0.0, 0.2]);
        assert_eq!(segments, vec![cue(0, 2_000, "hello")]);
    }

    #[test]
    fn an_unsure_attempt_is_decoded_again_hotter() {
        let mut script = Script::new([
            unsure(vec![at(0), 11, at(2_000)], -2.0),
            spoken(vec![at(0), 10, at(2_000)]),
        ]);

        let segments = run(&mut script, 3_000, Some(ZH));

        assert_eq!(script.temperatures(), vec![0.0, 0.2]);
        assert_eq!(segments, vec![cue(0, 2_000, "hello")]);
    }

    #[test]
    fn when_every_attempt_fails_the_surest_unrepeated_one_is_kept() {
        let mut script = Script::new([
            unsure(vec![at(0), 10, at(2_000)], -2.0),
            spoken(vec![at(0), 14, at(2_000)]),
            unsure(vec![at(0), 11, at(2_000)], -1.5),
            unsure(vec![at(0), 12, at(2_000)], -3.0),
            unsure(vec![at(0), 10, at(2_000)], -1.6),
            unsure(vec![at(0), 12, at(2_000)], -2.5),
        ]);

        let segments = run(&mut script, 3_000, Some(ZH));

        assert_eq!(script.temperatures(), vec![0.0, 0.2, 0.4, 0.6, 0.8, 1.0]);
        assert_eq!(segments, vec![cue(0, 2_000, "there")]);
    }

    #[test]
    fn when_every_attempt_fails_the_previous_text_is_dropped() {
        let mut script = Script::new([
            unsure(vec![at(0), 10, at(2_000), at(2_000), 12], -1.5),
            unsure(vec![at(0), 11, at(2_000)], -2.0),
            unsure(vec![at(0), 11, at(2_000)], -2.0),
            unsure(vec![at(0), 11, at(2_000)], -2.0),
            unsure(vec![at(0), 11, at(2_000)], -2.0),
            unsure(vec![at(0), 11, at(2_000)], -2.0),
            silence(),
        ]);

        let segments = run(&mut script, 3_000, Some(ZH));

        assert_eq!(segments, vec![cue(0, 2_000, "hello")]);
        assert_eq!(script.prompts()[6], vec![START_OF_TRANSCRIPT, ZH, TRANSCRIBE]);
    }

    #[test]
    fn when_every_attempt_repeats_the_surest_one_is_kept() {
        let scores = [-0.5, -0.3, -0.9, -0.1, -0.4, -0.6];
        let mut script = Script::new(
            (1..=6u32)
                .zip(scores)
                .map(|(end, score)| unsure(vec![at(0), 14, at(end * 1_000)], score)),
        );

        let segments = run(&mut script, 3_000, Some(ZH));

        assert_eq!(segments.iter().map(|segment| segment.end_ms).collect::<Vec<_>>(), vec![4_000]);
    }

    #[test]
    fn silence_is_skipped_without_another_attempt() {
        let mut script = Script::new([silence(), spoken(vec![at(0), 10, at(2_000)])]);

        let segments = run(&mut script, 6_000, Some(ZH));

        assert_eq!(script.loads, vec![0, 3_000]);
        assert_eq!(script.temperatures(), vec![0.0, 0.0]);
        assert_eq!(segments, vec![cue(30_000, 32_000, "hello")]);
    }

    #[test]
    fn a_likely_silence_with_confident_text_is_kept() {
        let mut script = Script::new([Attempt {
            tokens: vec![at(0), 10, at(2_000)],
            score: -0.1,
            no_speech_prob: 0.9,
        }]);

        let segments = run(&mut script, 3_000, Some(ZH));

        assert_eq!(script.temperatures(), vec![0.0]);
        assert_eq!(segments, vec![cue(0, 2_000, "hello")]);
    }

    #[test]
    fn the_previous_text_is_dropped_after_a_hot_attempt() {
        let mut script = Script::new([
            unsure(vec![at(0), 11, at(2_000)], -2.0),
            unsure(vec![at(0), 11, at(2_000)], -2.0),
            unsure(vec![at(0), 11, at(2_000)], -2.0),
            spoken(vec![at(0), 10, at(2_000), at(2_000), 12]),
            spoken(vec![at(0), 11, at(1_000), at(1_000), 12]),
            silence(),
        ]);

        let segments = run(&mut script, 3_000, Some(ZH));

        assert_eq!(segments, vec![cue(0, 2_000, "hello"), cue(2_000, 3_000, "there")]);
        assert_eq!(script.prompts()[4], vec![START_OF_TRANSCRIPT, ZH, TRANSCRIBE]);
        assert_eq!(
            script.prompts()[5],
            vec![START_OF_PREV, at(0), 11, at(1_000), START_OF_TRANSCRIPT, ZH, TRANSCRIBE]
        );
    }

    #[test]
    fn the_previous_text_stays_after_a_mild_attempt() {
        let mut script = Script::new([
            unsure(vec![at(0), 11, at(2_000)], -2.0),
            unsure(vec![at(0), 11, at(2_000)], -2.0),
            spoken(vec![at(0), 10, at(2_000), at(2_000), 12]),
            silence(),
        ]);

        run(&mut script, 3_000, Some(ZH));

        assert_eq!(script.temperatures(), vec![0.0, 0.2, 0.4, 0.0]);
        assert_eq!(
            script.prompts()[3],
            vec![START_OF_PREV, at(0), 10, at(2_000), START_OF_TRANSCRIPT, ZH, TRANSCRIBE]
        );
    }

    #[test]
    fn a_set_language_is_never_detected() {
        let mut script = Script::new([silence(), silence()]);

        run(&mut script, 6_000, Some(EN));

        assert!(script.detections.is_empty());
        assert!(script.prompts().iter().all(|prompt| prompt[1] == EN));
    }

    #[test]
    fn without_a_language_it_is_detected_every_four_minutes() {
        let mut script = Script::new(std::iter::repeat_with(silence).take(18));
        script.languages = VecDeque::from([ZH, EN, ZH]);

        run(&mut script, 54_000, None);

        assert_eq!(script.detections, vec![0, 24_000, 48_000]);
        let languages: Vec<u32> = script.prompts().iter().map(|prompt| prompt[1]).collect();
        assert_eq!(languages, [vec![ZH; 8], vec![EN; 8], vec![ZH; 2]].concat());
    }

    #[test]
    fn an_english_only_model_prompts_without_a_language() {
        let vocabulary = Vocabulary { multilingual: false, ..vocabulary() };
        let mut script = Script::new([silence(), silence()]);

        transcribe(&mut script, &vocabulary, 3_000, None).unwrap();
        transcribe(&mut script, &vocabulary, 3_000, Some(EN)).unwrap();

        assert!(script.detections.is_empty());
        assert_eq!(script.prompts(), vec![vec![START_OF_TRANSCRIPT], vec![START_OF_TRANSCRIPT]]);
    }

    #[test]
    fn long_lines_are_split_into_cues() {
        let mut script = Script::new([spoken(vec![at(0), 15, at(20_000)])]);

        let segments = run(&mut script, 3_000, Some(ZH));

        assert!(segments.len() > 1);
        assert_eq!(segments.first().unwrap().start_ms, 0);
        assert_eq!(segments.last().unwrap().end_ms, 20_000);
    }

    #[test]
    fn a_failure_stops_the_run() {
        let mut failing_load = Script::new([silence()]);
        failing_load.fail_load = true;
        let mut no_language = Script::new([silence()]);
        let mut out_of_attempts = Script::new([spoken(vec![at(0), 10, at(1_000), at(1_000)])]);

        assert_eq!(
            transcribe(&mut failing_load, &vocabulary(), 3_000, Some(ZH)),
            Err("load failed".to_owned())
        );
        assert_eq!(
            transcribe(&mut no_language, &vocabulary(), 3_000, None),
            Err("no language".to_owned())
        );
        assert_eq!(
            transcribe(&mut out_of_attempts, &vocabulary(), 6_000, Some(ZH)),
            Err("out of attempts".to_owned())
        );
        assert!(failing_load.decodes.is_empty());
        assert!(no_language.decodes.is_empty());
    }

    #[test]
    fn repeated_text_compresses_past_the_limit() {
        assert!(compression_ratio(&"la ".repeat(30)) > 2.4);
    }

    #[test]
    fn ordinary_text_stays_under_the_limit() {
        assert!(compression_ratio("真的是美啊，我们明天再见，路上小心。") < 2.4);
        assert!(compression_ratio(LONG_LINE) < 2.4);
    }

    #[test]
    fn the_average_log_probability_counts_the_end_token() {
        assert_eq!(average_log_prob(-0.5, 3), -0.375);
    }

    #[test]
    fn timestamp_tokens_are_named_by_their_time() {
        assert_eq!(timestamp_token(0), "<|0.00|>");
        assert_eq!(timestamp_token(62), "<|1.24|>");
        assert_eq!(timestamp_token(1_500), "<|30.00|>");
    }
}

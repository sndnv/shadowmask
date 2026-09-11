use crate::Segment;

const MAX_CUE_CHARS: usize = 84;
const MAX_CUE_MS: u64 = 6_000;
const MIN_PIECE_CHARS: usize = 24;

pub fn parse_segments(outputs: &[String], chunk_seconds: f64) -> Vec<Segment> {
    let mut segments = Vec::new();
    for (index, line) in outputs.iter().enumerate() {
        let offset = index as f64 * chunk_seconds;
        append_chunk(line, offset, chunk_seconds, &mut segments);
    }
    segments.into_iter().flat_map(split_segment).collect()
}

fn split_segment(segment: Segment) -> Vec<Segment> {
    let char_count = segment.text.chars().count();
    let duration = segment.end_ms.saturating_sub(segment.start_ms);
    let by_chars = char_count.div_ceil(MAX_CUE_CHARS);
    let by_duration = (duration as usize).div_ceil(MAX_CUE_MS as usize);
    let cap_by_min = (char_count / MIN_PIECE_CHARS).max(1);
    let target = by_chars.max(by_duration.min(cap_by_min)).max(1);
    if target <= 1 {
        return vec![segment];
    }
    let max_chars = char_count.div_ceil(target);
    let pieces = split_text(&segment.text, max_chars);
    distribute_time(&segment, &pieces)
}

fn distribute_time(segment: &Segment, pieces: &[String]) -> Vec<Segment> {
    let total: u64 = pieces.iter().map(|p| p.chars().count() as u64).sum();
    let span = segment.end_ms.saturating_sub(segment.start_ms);
    let last = pieces.len() - 1;
    let mut out = Vec::with_capacity(pieces.len());
    let mut cursor = segment.start_ms;
    let mut consumed = 0u64;
    for (index, piece) in pieces.iter().enumerate() {
        consumed += piece.chars().count() as u64;
        let end = if index == last {
            segment.end_ms
        } else {
            (segment.start_ms + span * consumed / total).max(cursor)
        };
        out.push(Segment { start_ms: cursor, end_ms: end, text: piece.clone() });
        cursor = end;
    }
    out
}

fn split_text(text: &str, max_chars: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut pieces = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        if chars.len() - start <= max_chars {
            push_piece(&chars[start..], &mut pieces);
            break;
        }
        let cut = break_point(&chars, start, start + max_chars);
        push_piece(&chars[start..cut], &mut pieces);
        start = cut;
    }
    pieces
}

fn push_piece(slice: &[char], out: &mut Vec<String>) {
    let piece = slice.iter().collect::<String>().trim().to_owned();
    if !piece.is_empty() {
        out.push(piece);
    }
}

fn break_point(chars: &[char], start: usize, window_end: usize) -> usize {
    for j in (start + 1..window_end).rev() {
        if is_sentence_end(chars[j]) {
            return j + 1;
        }
    }
    for j in (start + 1..window_end).rev() {
        if chars[j].is_whitespace() {
            return j + 1;
        }
    }
    window_end
}

fn is_sentence_end(c: char) -> bool {
    matches!(c, '.' | '!' | '?' | '。' | '！' | '？' | '…')
}

fn append_chunk(line: &str, offset: f64, chunk_seconds: f64, out: &mut Vec<Segment>) {
    let stamps = timestamp_positions(line);
    if stamps.len() < 2 {
        let text = strip_markers(line);
        if !text.is_empty() {
            out.push(Segment {
                start_ms: to_ms(offset),
                end_ms: to_ms(offset + chunk_seconds),
                text,
            });
        }
        return;
    }
    for pair in stamps.windows(2) {
        let (_, text_start, start_ts) = pair[0];
        let (text_end, _, end_ts) = pair[1];
        let text = strip_markers(&line[text_start..text_end]);
        if text.is_empty() {
            continue;
        }
        out.push(Segment {
            start_ms: to_ms(start_ts + offset),
            end_ms: to_ms(end_ts.max(start_ts) + offset),
            text,
        });
    }
}

fn to_ms(seconds: f64) -> u64 {
    (seconds * 1000.0) as u64
}

fn timestamp_positions(line: &str) -> Vec<(usize, usize, f64)> {
    let mut out = Vec::new();
    let mut search = 0;
    while let Some(rel) = line[search..].find("<|") {
        let open = search + rel;
        match line[open + 2..].find("|>") {
            Some(close_rel) => {
                let value_end = open + 2 + close_rel;
                let token_end = value_end + 2;
                if let Ok(value) = line[open + 2..value_end].trim().parse::<f64>() {
                    out.push((open, token_end, value));
                }
                search = token_end;
            }
            None => break,
        }
    }
    out
}

fn strip_markers(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find("<|") {
        out.push_str(&rest[..open]);
        match rest[open + 2..].find("|>") {
            Some(rel) => rest = &rest[open + 2 + rel + 2..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_a_chunk_into_individual_cues() {
        let outputs = vec!["<|0.00|> Hello there.<|2.00|><|2.00|> Second line.<|4.50|>".to_owned()];
        let segments = parse_segments(&outputs, 30.0);
        assert_eq!(
            segments,
            vec![
                Segment { start_ms: 0, end_ms: 2000, text: "Hello there.".to_owned() },
                Segment { start_ms: 2000, end_ms: 4500, text: "Second line.".to_owned() },
            ]
        );
    }

    #[test]
    fn offsets_later_chunks_by_their_position() {
        let outputs = vec![
            "<|0.00|> first chunk<|5.00|>".to_owned(),
            "<|1.00|> second chunk<|6.00|>".to_owned(),
            "<|2.00|> third chunk<|7.00|>".to_owned(),
        ];
        let segments = parse_segments(&outputs, 30.0);
        assert_eq!(segments[0].start_ms, 0);
        assert_eq!(segments[0].end_ms, 5_000);
        assert_eq!(segments[1].start_ms, 31_000);
        assert_eq!(segments[1].end_ms, 36_000);
        assert_eq!(segments[2].start_ms, 62_000);
        assert_eq!(segments[2].end_ms, 67_000);
        assert_eq!(segments[2].text, "third chunk");
    }

    #[test]
    fn skips_empty_spans_between_cues() {
        let outputs = vec!["<|0.00|> only text<|1.00|><|1.00|><|3.00|>".to_owned()];
        let segments = parse_segments(&outputs, 30.0);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "only text");
    }

    #[test]
    fn chunk_without_timestamps_spans_the_window() {
        let outputs = vec!["no timestamps here".to_owned()];
        let segments = parse_segments(&outputs, 30.0);
        assert_eq!(
            segments,
            vec![Segment { start_ms: 0, end_ms: 30_000, text: "no timestamps here".to_owned() }]
        );
    }

    #[test]
    fn untimestamped_later_chunk_uses_its_offset_window() {
        let outputs = vec!["<|0.00|> first<|2.00|>".to_owned(), "loose text".to_owned()];
        let segments = parse_segments(&outputs, 30.0);
        assert_eq!(segments[1].start_ms, 30_000);
        assert_eq!(segments[1].end_ms, 60_000);
        assert_eq!(segments[1].text, "loose text");
    }

    #[test]
    fn empty_chunk_yields_nothing() {
        let outputs = vec!["   ".to_owned()];
        assert!(parse_segments(&outputs, 30.0).is_empty());
    }

    #[test]
    fn strips_residual_non_timestamp_markers_from_text() {
        let outputs = vec!["<|0.00|> hi <|noise|> there<|1.00|>".to_owned()];
        let segments = parse_segments(&outputs, 30.0);
        assert_eq!(segments[0].text, "hi  there");
    }

    #[test]
    fn unterminated_marker_does_not_panic() {
        let outputs = vec!["<|0.00|> text then <|broken".to_owned()];
        let segments = parse_segments(&outputs, 30.0);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "text then");
    }

    #[test]
    fn end_before_start_is_clamped() {
        let outputs = vec!["<|5.00|> reversed<|1.00|>".to_owned()];
        let segments = parse_segments(&outputs, 30.0);
        assert_eq!(segments[0].start_ms, 5_000);
        assert_eq!(segments[0].end_ms, 5_000);
    }

    #[test]
    fn splits_a_long_cue_into_readable_pieces() {
        let long = "This is the first sentence. This is the second sentence which is a \
            bit longer. And here is a third sentence to push things well past the limit.";
        let outputs = vec![format!("<|0.00|> {long}<|20.00|>")];
        let segments = parse_segments(&outputs, 30.0);
        assert!(segments.len() > 1);
        for segment in &segments {
            assert!(segment.text.chars().count() <= MAX_CUE_CHARS);
        }
        assert_eq!(segments.first().unwrap().start_ms, 0);
        assert_eq!(segments.last().unwrap().end_ms, 20_000);
        for pair in segments.windows(2) {
            assert_eq!(pair[0].end_ms, pair[1].start_ms);
            assert!(pair[0].end_ms >= pair[0].start_ms);
        }
    }

    #[test]
    fn does_not_split_a_short_cue() {
        let outputs = vec!["<|0.00|> Short and sweet.<|3.00|>".to_owned()];
        let segments = parse_segments(&outputs, 30.0);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "Short and sweet.");
    }

    #[test]
    fn split_text_prefers_sentence_boundaries() {
        let pieces = split_text("First one. Second two.", 14);
        assert_eq!(pieces, vec!["First one.".to_owned(), "Second two.".to_owned()]);
    }

    #[test]
    fn split_text_hard_splits_runs_without_boundaries() {
        let pieces = split_text("abcdefghijklmnop", 5);
        assert_eq!(pieces, vec!["abcde", "fghij", "klmno", "p"]);
    }

    #[test]
    fn split_text_skips_whitespace_only_windows() {
        let pieces = split_text("word    word", 4);
        assert_eq!(pieces, vec!["word".to_owned(), "word".to_owned()]);
    }

    #[test]
    fn distribute_time_splits_span_by_character_share() {
        let segment = Segment { start_ms: 0, end_ms: 10_000, text: String::new() };
        let pieces = vec!["a".repeat(10), "b".repeat(90)];
        let out = distribute_time(&segment, &pieces);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].start_ms, 0);
        assert_eq!(out[0].end_ms, 1_000);
        assert_eq!(out[1].start_ms, 1_000);
        assert_eq!(out[1].end_ms, 10_000);
    }
}

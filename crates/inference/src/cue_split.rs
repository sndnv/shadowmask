use crate::Segment;

const MAX_CUE_CHARS: usize = 84;
const MAX_CUE_MS: u64 = 6_000;
const MIN_PIECE_CHARS: usize = 24;

pub fn split_cue(segment: Segment) -> Vec<Segment> {
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
        if is_sentence_end(chars[j]) && !splits_a_number(chars, j) {
            return j + 1;
        }
    }
    for j in (start + 1..window_end).rev() {
        if chars[j].is_whitespace() {
            return j + 1;
        }
    }
    for j in (start + 1..window_end).rev() {
        if is_clause_end(chars[j]) && !splits_a_number(chars, j) {
            return j + 1;
        }
    }
    window_end
}

fn is_sentence_end(c: char) -> bool {
    matches!(c, '.' | '!' | '?' | '。' | '！' | '？' | '…')
}

fn is_clause_end(c: char) -> bool {
    matches!(c, ',' | '，' | '、' | ';' | '；' | ':' | '：')
}

fn splits_a_number(chars: &[char], index: usize) -> bool {
    matches!(chars[index], '.' | ',' | ';' | ':')
        && chars[index - 1].is_ascii_digit()
        && chars.get(index + 1).is_some_and(char::is_ascii_digit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_a_long_cue_into_readable_pieces() {
        let long = "This is the first sentence. This is the second sentence which is a \
            bit longer. And here is a third sentence to push things well past the limit.";
        let segments = split_cue(Segment { start_ms: 0, end_ms: 20_000, text: long.to_owned() });
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
        let cue = Segment { start_ms: 0, end_ms: 3_000, text: "Short and sweet.".to_owned() };
        assert_eq!(split_cue(cue.clone()), vec![cue]);
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
    fn split_text_breaks_after_a_comma_when_there_is_no_space() {
        let pieces = split_text("今天早上天气很好,我们决定去山上走一走,路上的人不多", 12);
        assert_eq!(pieces, vec!["今天早上天气很好,", "我们决定去山上走一走,", "路上的人不多"]);
    }

    #[test]
    fn split_text_breaks_after_chinese_clause_marks() {
        let pieces = split_text("味道很鲜、一点也不腥，真好吃", 8);
        assert_eq!(pieces, vec!["味道很鲜、", "一点也不腥，", "真好吃"]);
    }

    #[test]
    fn split_text_breaks_after_semicolons_and_colons() {
        let pieces = split_text("第一点；第二点：第三点;第四点:最后", 4);
        assert_eq!(pieces, vec!["第一点；", "第二点：", "第三点;", "第四点:", "最后"]);
    }

    #[test]
    fn split_text_keeps_numbers_whole() {
        assert_eq!(
            split_text("今天花了1,000块钱买了很多东西和水果", 12),
            vec!["今天花了1,000块钱买", "了很多东西和水果"]
        );
        assert_eq!(
            split_text("会议在10:30开始,大家准时到", 8),
            vec!["会议在10:30", "开始,大家准时到"]
        );
        assert_eq!(
            split_text("The price went up 3.5 percent today", 25),
            vec!["The price went up 3.5", "percent today"]
        );
        assert_eq!(split_text("经济增长,今年增长了3.5个百分点", 13)[0], "经济增长,");
        assert_eq!(split_text("价格:100元的东西很好", 5)[0], "价格:");
        assert_eq!(split_text("我买了3,他买了4个苹果", 7)[0], "我买了3,");
    }

    #[test]
    fn split_text_prefers_a_space_to_a_comma() {
        let pieces = split_text("Well, I went to the store and bought apples", 30);
        assert_eq!(pieces, vec!["Well, I went to the store and", "bought apples"]);
    }

    #[test]
    fn a_long_chinese_line_splits_at_its_commas() {
        let line = "今天早上天气很好,我们决定去山上走一走,路上的人不多,空气也特别新鲜,我们先在山脚下的\
            小店买了两瓶水和一些面包,然后沿着石头小路慢慢往上爬,走了大概一个小时,我们在半山腰的亭子里\
            休息了一会儿,看着远处的湖水和城市,心情一下子就放松了。";

        let segments = split_cue(Segment { start_ms: 0, end_ms: 26_240, text: line.to_owned() });

        assert_eq!(segments.len(), 5);
        let (last, rest) = segments.split_last().unwrap();
        assert!(rest.iter().all(|segment| segment.text.ends_with(',')));
        assert!(last.text.ends_with('。'));
        assert_eq!(segments.iter().map(|segment| segment.text.as_str()).collect::<String>(), line);
        assert_eq!((segments[0].start_ms, last.end_ms), (0, 26_240));
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

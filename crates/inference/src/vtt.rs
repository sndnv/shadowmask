use crate::Segment;

pub fn segments_to_vtt(segments: &[Segment]) -> String {
    let mut out = String::from("WEBVTT\n");
    for segment in segments {
        let text = segment.text.trim();
        if text.is_empty() {
            continue;
        }
        out.push('\n');
        out.push_str(&format!(
            "{} --> {}\n",
            format_timestamp(segment.start_ms),
            format_timestamp(segment.end_ms)
        ));
        out.push_str(text);
        out.push('\n');
    }
    out
}

fn format_timestamp(ms: u64) -> String {
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1000;
    let millis = ms % 1000;
    format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(start_ms: u64, end_ms: u64, text: &str) -> Segment {
        Segment { start_ms, end_ms, text: text.to_owned() }
    }

    #[test]
    fn empty_input_is_header_only() {
        assert_eq!(segments_to_vtt(&[]), "WEBVTT\n");
    }

    #[test]
    fn renders_cues_with_timestamps() {
        let vtt =
            segments_to_vtt(&[segment(0, 2000, " Hello "), segment(3_661_500, 3_662_000, "later")]);
        assert_eq!(
            vtt,
            "WEBVTT\n\n00:00:00.000 --> 00:00:02.000\nHello\n\n01:01:01.500 --> 01:01:02.000\nlater\n"
        );
    }

    #[test]
    fn skips_blank_segments() {
        let vtt = segments_to_vtt(&[segment(0, 1000, "   ")]);
        assert_eq!(vtt, "WEBVTT\n");
    }
}

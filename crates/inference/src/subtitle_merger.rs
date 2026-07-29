use domain::error::SubtitleError;
use domain::media::{FetchedSubtitle, SubtitleCombiner, SubtitleFormat};
use subtp::srt::SubRip;
use subtp::vtt::{VttBlock, WebVtt};

use crate::{Segment, segments_to_vtt};

pub struct SubtitleMerger;

struct Cue {
    start_ms: u64,
    end_ms: u64,
    lines: Vec<String>,
}

impl SubtitleCombiner for SubtitleMerger {
    fn combine(
        &self,
        primary: &FetchedSubtitle,
        secondary: &FetchedSubtitle,
    ) -> Result<FetchedSubtitle, SubtitleError> {
        let primary_cues = parse(primary)?;
        let secondary_cues = parse(secondary)?;

        let segments: Vec<Segment> = primary_cues
            .iter()
            .map(|cue| {
                let mut lines = cue.lines.clone();
                for secondary in &secondary_cues {
                    if secondary.start_ms < cue.end_ms && secondary.end_ms > cue.start_ms {
                        lines.extend(secondary.lines.iter().cloned());
                    }
                }
                Segment {
                    start_ms: cue.start_ms,
                    end_ms: cue.end_ms,
                    text: lines.join("\n"),
                }
            })
            .collect();

        Ok(FetchedSubtitle {
            content: segments_to_vtt(&segments),
            format: SubtitleFormat::Vtt,
        })
    }
}

fn parse(subtitle: &FetchedSubtitle) -> Result<Vec<Cue>, SubtitleError> {
    match subtitle.format {
        SubtitleFormat::Vtt => {
            let vtt = WebVtt::parse(subtitle.content.as_str())
                .map_err(|e| SubtitleError::Parse(format!("parse vtt: {e}")))?;
            Ok(vtt
                .blocks
                .into_iter()
                .filter_map(|block| match block {
                    VttBlock::Que(cue) => Some(Cue {
                        start_ms: ms(
                            cue.timings.start.hours,
                            cue.timings.start.minutes,
                            cue.timings.start.seconds,
                            cue.timings.start.milliseconds,
                        ),
                        end_ms: ms(
                            cue.timings.end.hours,
                            cue.timings.end.minutes,
                            cue.timings.end.seconds,
                            cue.timings.end.milliseconds,
                        ),
                        lines: cue.payload,
                    }),
                    _ => None,
                })
                .collect())
        }
        SubtitleFormat::Srt => {
            let srt = SubRip::parse(subtitle.content.as_str())
                .map_err(|e| SubtitleError::Parse(format!("parse srt: {e}")))?;
            Ok(srt
                .subtitles
                .into_iter()
                .map(|sub| Cue {
                    start_ms: ms(
                        sub.start.hours,
                        sub.start.minutes,
                        sub.start.seconds,
                        sub.start.milliseconds,
                    ),
                    end_ms: ms(
                        sub.end.hours,
                        sub.end.minutes,
                        sub.end.seconds,
                        sub.end.milliseconds,
                    ),
                    lines: sub.text,
                })
                .collect())
        }
        other => Err(SubtitleError::Backend(format!(
            "unsupported subtitle format: {other:?}"
        ))),
    }
}

fn ms(hours: u8, minutes: u8, seconds: u8, milliseconds: u16) -> u64 {
    hours as u64 * 3_600_000
        + minutes as u64 * 60_000
        + seconds as u64 * 1_000
        + milliseconds as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vtt(content: &str) -> FetchedSubtitle {
        FetchedSubtitle {
            content: content.to_owned(),
            format: SubtitleFormat::Vtt,
        }
    }

    fn srt(content: &str) -> FetchedSubtitle {
        FetchedSubtitle {
            content: content.to_owned(),
            format: SubtitleFormat::Srt,
        }
    }

    #[test]
    fn overlapping_secondary_cue_stacks_as_extra_lines() {
        let primary = vtt("WEBVTT\n\n00:00:00.000 --> 00:00:02.000\nHello\n");
        let secondary = vtt("WEBVTT\n\n00:00:00.500 --> 00:00:01.500\nBonjour\n");

        let combined = SubtitleMerger.combine(&primary, &secondary).unwrap();

        assert_eq!(combined.format, SubtitleFormat::Vtt);
        assert!(combined.content.contains("00:00:00.000 --> 00:00:02.000"));
        assert!(combined.content.contains("Hello\nBonjour"));
    }

    #[test]
    fn disjoint_primary_cue_stays_single_line() {
        let primary = vtt("WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nHello\n");
        let secondary = vtt("WEBVTT\n\n00:00:05.000 --> 00:00:06.000\nBonjour\n");

        let combined = SubtitleMerger.combine(&primary, &secondary).unwrap();

        assert!(combined.content.contains("Hello"));
        assert!(!combined.content.contains("Bonjour"));
    }

    #[test]
    fn empty_primary_is_header_only() {
        let primary = vtt("WEBVTT\n\n");
        let secondary = vtt("WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nBonjour\n");

        let combined = SubtitleMerger.combine(&primary, &secondary).unwrap();

        assert_eq!(combined.content, "WEBVTT\n");
    }

    #[test]
    fn merges_srt_primary_with_srt_secondary() {
        let primary = srt("1\n00:00:00,000 --> 00:00:02,000\nHello\n");
        let secondary = srt("1\n00:00:00,000 --> 00:00:02,000\nBonjour\n");

        let combined = SubtitleMerger.combine(&primary, &secondary).unwrap();

        assert_eq!(combined.format, SubtitleFormat::Vtt);
        assert!(combined.content.contains("Hello\nBonjour"));
    }

    #[test]
    fn merges_srt_primary_with_vtt_secondary() {
        let primary = srt("1\n00:00:00,000 --> 00:00:02,000\nHello\n");
        let secondary = vtt("WEBVTT\n\n00:00:01.000 --> 00:00:03.000\nBonjour\n");

        let combined = SubtitleMerger.combine(&primary, &secondary).unwrap();

        assert!(combined.content.contains("Hello\nBonjour"));
    }

    #[test]
    fn malformed_primary_is_parse_error() {
        let primary = srt("@@@ not an srt @@@");
        let secondary = vtt("WEBVTT\n");

        assert!(matches!(
            SubtitleMerger.combine(&primary, &secondary).unwrap_err(),
            SubtitleError::Parse(_)
        ));
    }

    #[test]
    fn unsupported_format_is_backend_error() {
        let primary = FetchedSubtitle {
            content: String::new(),
            format: SubtitleFormat::Ass,
        };
        let secondary = vtt("WEBVTT\n");

        assert!(matches!(
            SubtitleMerger.combine(&primary, &secondary).unwrap_err(),
            SubtitleError::Backend(_)
        ));
    }
}

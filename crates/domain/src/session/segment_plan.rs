#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub start_ms: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentPlan {
    pub segments: Vec<Segment>,
}

impl SegmentPlan {
    pub fn index_at(&self, position_ms: u64) -> usize {
        self.segments
            .iter()
            .rposition(|s| s.start_ms <= position_ms)
            .unwrap_or(0)
    }

    pub fn from_index(&self, first: usize) -> SegmentPlan {
        SegmentPlan {
            segments: self.segments.get(first..).unwrap_or_default().to_vec(),
        }
    }

    pub fn start_of(&self, index: usize) -> u64 {
        self.segments.get(index).map_or(0, |s| s.start_ms)
    }
}

pub fn plan_segments(keyframes_ms: &[u64], duration_ms: u64, target_ms: u64) -> SegmentPlan {
    if duration_ms == 0 {
        return SegmentPlan {
            segments: Vec::new(),
        };
    }
    let target = target_ms.max(1);
    let mut boundaries: Vec<u64> = keyframes_ms
        .iter()
        .copied()
        .filter(|&k| k > 0 && k < duration_ms)
        .collect();
    boundaries.sort_unstable();
    boundaries.dedup();

    let mut segments = Vec::new();
    let mut start = 0u64;
    for boundary in boundaries {
        if boundary - start >= target {
            segments.push(Segment {
                start_ms: start,
                duration_ms: boundary - start,
            });
            start = boundary;
        }
    }
    segments.push(Segment {
        start_ms: start,
        duration_ms: duration_ms - start,
    });
    SegmentPlan { segments }
}

pub fn plan_segments_on_grid(
    keyframes_ms: &[u64],
    duration_ms: u64,
    target_ms: u64,
) -> SegmentPlan {
    if duration_ms == 0 {
        return SegmentPlan {
            segments: Vec::new(),
        };
    }
    let target = target_ms.max(1);
    let mut keyframes: Vec<u64> = keyframes_ms
        .iter()
        .copied()
        .filter(|&k| k < duration_ms)
        .collect();
    keyframes.sort_unstable();
    keyframes.dedup();
    let origin = keyframes.first().copied().unwrap_or(0);

    let mut boundaries: Vec<u64> = Vec::new();
    let mut mark = origin.saturating_add(target);
    while mark < duration_ms {
        match keyframes.iter().copied().find(|&k| k >= mark) {
            Some(cut) if cut > boundaries.last().copied().unwrap_or(origin) => {
                boundaries.push(cut);
            }
            _ => {}
        }
        mark = mark.saturating_add(target);
    }

    let mut segments = Vec::new();
    let mut start = 0u64;
    for boundary in boundaries {
        segments.push(Segment {
            start_ms: start,
            duration_ms: boundary - start,
        });
        start = boundary;
    }
    segments.push(Segment {
        start_ms: start,
        duration_ms: duration_ms - start,
    });
    SegmentPlan { segments }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn starts(plan: &SegmentPlan) -> Vec<u64> {
        plan.segments.iter().map(|s| s.start_ms).collect()
    }

    fn durations(plan: &SegmentPlan) -> Vec<u64> {
        plan.segments.iter().map(|s| s.duration_ms).collect()
    }

    #[test]
    fn zero_duration_yields_no_segments() {
        assert!(plan_segments(&[4_000], 0, 4_000).segments.is_empty());
    }

    #[test]
    fn no_keyframes_is_a_single_segment() {
        let plan = plan_segments(&[], 10_000, 4_000);
        assert_eq!(starts(&plan), vec![0]);
        assert_eq!(durations(&plan), vec![10_000]);
    }

    #[test]
    fn keyframe_at_zero_is_ignored() {
        let plan = plan_segments(&[0], 10_000, 4_000);
        assert_eq!(durations(&plan), vec![10_000]);
    }

    #[test]
    fn cuts_on_keyframes_at_least_target_apart() {
        let plan = plan_segments(&[4_000, 8_000, 12_000], 15_000, 4_000);
        assert_eq!(starts(&plan), vec![0, 4_000, 8_000, 12_000]);
        assert_eq!(durations(&plan), vec![4_000, 4_000, 4_000, 3_000]);
    }

    #[test]
    fn coalesces_keyframes_closer_than_target() {
        let plan = plan_segments(&[1_000, 2_000, 5_000, 6_000], 8_000, 4_000);
        assert_eq!(starts(&plan), vec![0, 5_000]);
        assert_eq!(durations(&plan), vec![5_000, 3_000]);
    }

    #[test]
    fn keyframes_beyond_duration_are_filtered() {
        let plan = plan_segments(&[4_000, 20_000], 8_000, 3_000);
        assert_eq!(starts(&plan), vec![0, 4_000]);
        assert_eq!(durations(&plan), vec![4_000, 4_000]);
    }

    #[test]
    fn duplicate_keyframes_are_deduped() {
        let plan = plan_segments(&[4_000, 4_000, 4_000], 8_000, 3_000);
        assert_eq!(starts(&plan), vec![0, 4_000]);
    }

    #[test]
    fn unsorted_keyframes_are_sorted() {
        let plan = plan_segments(&[8_000, 4_000], 12_000, 4_000);
        assert_eq!(starts(&plan), vec![0, 4_000, 8_000]);
    }

    #[test]
    fn target_zero_is_treated_as_one() {
        let plan = plan_segments(&[1, 2, 3], 5, 0);
        assert_eq!(starts(&plan), vec![0, 1, 2, 3]);
        assert_eq!(durations(&plan), vec![1, 1, 1, 2]);
    }

    #[test]
    fn index_at_finds_the_segment_holding_a_position() {
        let plan = plan_segments_on_grid(&[0, 4_950, 8_384, 14_166], 20_000, 4_000);
        assert_eq!(plan.index_at(0), 0);
        assert_eq!(plan.index_at(4_949), 0);
        assert_eq!(
            plan.index_at(4_950),
            1,
            "a boundary belongs to the segment it opens"
        );
        assert_eq!(plan.index_at(8_000), 1);
        assert_eq!(plan.index_at(19_999), 3);
    }

    #[test]
    fn index_at_beyond_the_end_stays_on_the_last_segment() {
        let plan = plan_segments_on_grid(&[0, 4_950], 10_000, 4_000);
        assert_eq!(plan.index_at(u64::MAX), plan.segments.len() - 1);
    }

    #[test]
    fn index_at_of_an_empty_plan_is_zero() {
        let plan = SegmentPlan {
            segments: Vec::new(),
        };
        assert_eq!(plan.index_at(5_000), 0);
        assert_eq!(plan.start_of(0), 0);
    }

    #[test]
    fn from_index_keeps_the_tail_and_its_absolute_starts() {
        let plan = plan_segments_on_grid(&[0, 4_950, 8_384, 14_166], 20_000, 4_000);
        let tail = plan.from_index(2);
        assert_eq!(tail.segments.len(), plan.segments.len() - 2);
        assert_eq!(
            tail.segments[0].start_ms, plan.segments[2].start_ms,
            "the tail keeps absolute positions so segment names still line up"
        );
        assert_eq!(plan.start_of(2), plan.segments[2].start_ms);
    }

    #[test]
    fn from_index_past_the_end_is_empty_rather_than_a_panic() {
        let plan = plan_segments_on_grid(&[0, 4_950], 10_000, 4_000);
        assert!(plan.from_index(99).segments.is_empty());
    }

    #[test]
    fn the_grid_plan_reproduces_ffmpeg_on_the_4k60_sample() {
        let keyframes = [
            0, 500, 2_783, 4_950, 5_767, 8_384, 10_534, 11_333, 14_166, 16_233, 17_033, 18_733,
            22_733, 23_366, 25_533, 27_700, 28_500, 29_900, 32_267, 33_467, 34_267, 35_534, 37_034,
            38_050, 39_667, 40_350, 42_884,
        ];
        let plan = plan_segments_on_grid(&keyframes, 45_198, 4_000);
        assert_eq!(
            starts(&plan),
            vec![
                0, 4_950, 8_384, 14_166, 16_233, 22_733, 25_533, 28_500, 32_267, 37_034, 40_350
            ],
            "these are the boundaries ffmpeg's hls muxer actually produced for this file, \
             measured with hls_time 4; the planner must agree or the playlist lies"
        );
    }

    #[test]
    fn the_grid_cuts_on_absolute_marks_not_since_the_last_cut() {
        let plan = plan_segments_on_grid(&[0, 4_950, 8_384, 10_534], 20_000, 4_000);
        assert!(
            starts(&plan).contains(&8_384),
            "8384 follows 4950 by only 3.4s, but it is the first keyframe past the 8s mark, \
             so ffmpeg cuts there and so must we"
        );
    }

    #[test]
    fn the_grid_plan_covers_the_whole_timeline_without_gaps() {
        let plan = plan_segments_on_grid(&[0, 3_000, 7_000, 11_000], 15_000, 4_000);
        let mut acc = 0;
        for segment in &plan.segments {
            assert_eq!(segment.start_ms, acc);
            assert!(segment.duration_ms > 0);
            acc += segment.duration_ms;
        }
        assert_eq!(acc, 15_000);
    }

    #[test]
    fn the_grid_plan_of_zero_duration_is_empty() {
        assert!(
            plan_segments_on_grid(&[4_000], 0, 4_000)
                .segments
                .is_empty()
        );
    }

    #[test]
    fn the_grid_plan_without_keyframes_is_a_single_segment() {
        let plan = plan_segments_on_grid(&[], 10_000, 4_000);
        assert_eq!(starts(&plan), vec![0]);
        assert_eq!(durations(&plan), vec![10_000]);
    }

    #[test]
    fn the_grid_marks_run_from_the_first_keyframe_not_from_zero() {
        let plan = plan_segments_on_grid(&[23, 2_523, 5_023, 7_523, 10_023], 12_000, 2_500);
        assert_eq!(
            starts(&plan),
            vec![0, 2_523, 5_023, 7_523, 10_023],
            "ffmpeg measures from the first packet, so a file that does not start at zero \
             still cuts on its own grid"
        );
    }

    #[test]
    fn the_grid_plan_never_repeats_a_boundary_when_marks_share_a_keyframe() {
        let plan = plan_segments_on_grid(&[0, 1_000, 20_000], 25_000, 4_000);
        let boundaries = starts(&plan);
        let mut deduped = boundaries.clone();
        deduped.dedup();
        assert_eq!(
            boundaries, deduped,
            "several 4s marks fall before the 20s keyframe and must not each emit it"
        );
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn plan_is_contiguous_and_covers_the_full_duration(
            keyframes in prop::collection::vec(0u64..30_000, 0..20),
            duration in 1u64..30_000,
            target in 1u64..8_000,
        ) {
            let plan = plan_segments(&keyframes, duration, target);
            prop_assert!(!plan.segments.is_empty());
            prop_assert_eq!(plan.segments[0].start_ms, 0);
            let mut acc = 0u64;
            for (i, segment) in plan.segments.iter().enumerate() {
                prop_assert!(segment.duration_ms > 0);
                prop_assert_eq!(segment.start_ms, acc);
                acc += segment.duration_ms;
                if i > 0 {
                    prop_assert!(keyframes.contains(&segment.start_ms));
                }
                if i + 1 < plan.segments.len() {
                    prop_assert!(segment.duration_ms >= target);
                }
            }
            prop_assert_eq!(acc, duration);
        }
    }
}

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
        self.segments.iter().rposition(|s| s.start_ms <= position_ms).unwrap_or(0)
    }

    pub fn from_index(&self, first: usize) -> SegmentPlan {
        SegmentPlan { segments: self.segments.get(first..).unwrap_or_default().to_vec() }
    }

    pub fn start_of(&self, index: usize) -> u64 {
        self.segments.get(index).map_or(0, |s| s.start_ms)
    }
}

pub fn plan_fixed_segments(duration_ms: u64, target_ms: u64) -> SegmentPlan {
    let target = target_ms.max(1);
    let mut segments: Vec<Segment> = Vec::new();
    let mut start = 0u64;
    while start < duration_ms {
        let remaining = duration_ms - start;
        match segments.last_mut() {
            Some(last) if remaining < target => last.duration_ms += remaining,
            _ => segments.push(Segment { start_ms: start, duration_ms: target.min(remaining) }),
        }
        start += target;
    }
    SegmentPlan { segments }
}

pub fn plan_cuts(
    keyframes_us: &[u64],
    from_us: u64,
    duration_ms: u64,
    target_ms: u64,
) -> SegmentPlan {
    let duration_us = duration_ms.saturating_mul(1_000);
    if from_us >= duration_us {
        return SegmentPlan { segments: Vec::new() };
    }
    let target_us = target_ms.max(1).saturating_mul(1_000);
    let mut keyframes: Vec<u64> =
        keyframes_us.iter().copied().filter(|&k| k >= from_us && k < duration_us).collect();
    keyframes.sort_unstable();
    keyframes.dedup();
    let anchor = keyframes.first().copied().unwrap_or(from_us);

    let mut starts = vec![from_us];
    for keyframe in keyframes {
        let mark = (starts.len() as u64).saturating_mul(target_us);
        if keyframe - anchor >= mark {
            starts.push(keyframe);
        }
    }
    let mut bounds: Vec<u64> = starts.iter().map(|us| us / 1_000).collect();
    bounds.push(duration_ms);
    let segments = bounds
        .windows(2)
        .filter(|pair| pair[1] > pair[0])
        .map(|pair| Segment { start_ms: pair[0], duration_ms: pair[1] - pair[0] })
        .collect();
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

    fn us(keyframes_ms: &[u64]) -> Vec<u64> {
        keyframes_ms.iter().map(|ms| ms * 1_000).collect()
    }

    fn cuts(keyframes_ms: &[u64], from_ms: u64, duration_ms: u64, target_ms: u64) -> SegmentPlan {
        plan_cuts(&us(keyframes_ms), from_ms * 1_000, duration_ms, target_ms)
    }

    #[test]
    fn zero_duration_yields_no_segments() {
        assert!(cuts(&[4_000], 0, 0, 4_000).segments.is_empty());
        assert!(plan_fixed_segments(0, 4_000).segments.is_empty());
    }

    #[test]
    fn a_start_at_or_past_the_end_yields_no_segments() {
        assert!(cuts(&[0, 4_000], 10_000, 10_000, 4_000).segments.is_empty());
        assert!(cuts(&[0, 4_000], 12_000, 10_000, 4_000).segments.is_empty());
    }

    #[test]
    fn a_keyframe_less_than_a_millisecond_short_of_its_mark_is_not_a_cut() {
        let plan = plan_cuts(&[0, 3_999_600, 4_000_400, 9_000_000], 0, 12_000, 4_000);
        assert_eq!(
            starts(&plan),
            vec![0, 4_000, 9_000],
            "3.9996 s is short of the 4 s mark although it rounds to it in whole milliseconds"
        );
        assert_eq!(durations(&plan), vec![4_000, 5_000, 3_000]);
    }

    #[test]
    fn a_start_is_the_millisecond_its_keyframe_falls_in() {
        let plan = plan_cuts(&[8_007_999, 12_011_999], 8_007_999, 16_000, 4_000);
        assert_eq!(starts(&plan), vec![8_007, 12_011]);
        assert_eq!(durations(&plan), vec![4_004, 3_989]);
    }

    #[test]
    fn two_cuts_in_the_same_millisecond_leave_no_empty_segment() {
        let plan = plan_cuts(&[0, 2_100, 2_200], 0, 5, 1);
        assert_eq!(starts(&plan), vec![0, 2]);
        assert_eq!(durations(&plan), vec![2, 3]);
    }

    #[test]
    fn a_fixed_plan_folds_a_remainder_shorter_than_the_target_into_its_last_segment() {
        let plan = plan_fixed_segments(15_000, 4_000);
        assert_eq!(
            starts(&plan),
            vec![0, 4_000, 8_000],
            "the last video frame can fall before the final mark, and then ffmpeg never cuts there"
        );
        assert_eq!(durations(&plan), vec![4_000, 4_000, 7_000]);
    }

    #[test]
    fn a_film_shorter_than_the_target_is_one_segment() {
        assert_eq!(durations(&plan_fixed_segments(3_000, 4_000)), vec![3_000]);
    }

    #[test]
    fn a_fixed_plan_of_an_exact_multiple_has_no_empty_tail() {
        let plan = plan_fixed_segments(8_000, 4_000);
        assert_eq!(durations(&plan), vec![4_000, 4_000]);
    }

    #[test]
    fn a_fixed_plan_survives_a_zero_target() {
        assert_eq!(durations(&plan_fixed_segments(3, 0)), vec![1, 1, 1]);
    }

    #[test]
    fn no_keyframes_is_a_single_segment() {
        let plan = cuts(&[], 0, 10_000, 4_000);
        assert_eq!(starts(&plan), vec![0]);
        assert_eq!(durations(&plan), vec![10_000]);
    }

    #[test]
    fn keyframes_beyond_the_duration_are_ignored() {
        let plan = cuts(&[0, 4_000, 20_000], 0, 8_000, 3_000);
        assert_eq!(starts(&plan), vec![0, 4_000]);
        assert_eq!(durations(&plan), vec![4_000, 4_000]);
    }

    #[test]
    fn duplicate_and_unsorted_keyframes_are_tidied() {
        let plan = cuts(&[8_000, 0, 4_000, 4_000], 0, 12_000, 4_000);
        assert_eq!(starts(&plan), vec![0, 4_000, 8_000]);
    }

    #[test]
    fn a_zero_target_is_treated_as_one() {
        let plan = cuts(&[0, 1, 2, 3], 0, 5, 0);
        assert_eq!(starts(&plan), vec![0, 1, 2, 3]);
        assert_eq!(durations(&plan), vec![1, 1, 1, 2]);
    }

    #[test]
    fn a_long_gop_does_not_use_up_the_marks_it_spans() {
        let keyframes = [0, 9_000, 10_000, 11_000, 20_000, 21_000, 30_000];
        let plan = cuts(&keyframes, 0, 40_000, 4_000);
        assert_eq!(
            starts(&plan),
            vec![0, 9_000, 10_000, 20_000, 21_000, 30_000],
            "ffmpeg 9.0.2 and 5.1.9 cut this file at these keyframes with hls_time 4"
        );
    }

    #[test]
    fn a_restarted_run_counts_its_marks_from_its_own_first_keyframe() {
        let keyframes = [0, 9_000, 10_000, 11_000, 20_000, 21_000, 30_000];
        let plan = cuts(&keyframes, 10_000, 40_000, 4_000);
        assert_eq!(starts(&plan), vec![10_000, 20_000, 21_000, 30_000]);
        assert_eq!(durations(&plan), vec![10_000, 1_000, 9_000, 10_000]);
    }

    #[test]
    fn the_marks_run_from_the_first_keyframe_not_from_the_start() {
        let plan = cuts(&[23, 2_523, 5_023, 7_523, 10_023], 0, 12_000, 2_500);
        assert_eq!(starts(&plan), vec![0, 2_523, 5_023, 7_523, 10_023]);
    }

    #[test]
    fn the_cuts_reproduce_ffmpeg_on_the_4k60_sample() {
        let keyframes = [
            0, 500, 2_783, 4_950, 5_767, 8_384, 10_534, 11_333, 14_166, 16_233, 17_033, 18_733,
            22_733, 23_366, 25_533, 27_700, 28_500, 29_900, 32_267, 33_467, 34_267, 35_534, 37_034,
            38_050, 39_667, 40_350, 42_884,
        ];
        let plan = cuts(&keyframes, 0, 45_198, 4_000);
        assert_eq!(
            starts(&plan),
            vec![0, 4_950, 8_384, 14_166, 16_233, 22_733, 25_533, 28_500, 32_267, 37_034, 40_350],
            "the boundaries ffmpeg's hls muxer produced for this file with hls_time 4"
        );
    }

    #[test]
    fn a_keyframe_never_ends_two_segments() {
        let plan = cuts(&[0, 1_000, 20_000], 0, 25_000, 4_000);
        assert_eq!(starts(&plan), vec![0, 20_000]);
    }

    #[test]
    fn index_at_finds_the_segment_holding_a_position() {
        let plan = cuts(&[0, 4_950, 8_384, 14_166], 0, 20_000, 4_000);
        assert_eq!(plan.index_at(0), 0);
        assert_eq!(plan.index_at(4_949), 0);
        assert_eq!(plan.index_at(4_950), 1, "a boundary belongs to the segment it opens");
        assert_eq!(plan.index_at(8_000), 1);
        assert_eq!(plan.index_at(19_999), 3);
    }

    #[test]
    fn index_at_beyond_the_end_stays_on_the_last_segment() {
        let plan = cuts(&[0, 4_950], 0, 10_000, 4_000);
        assert_eq!(plan.index_at(u64::MAX), plan.segments.len() - 1);
    }

    #[test]
    fn index_at_of_an_empty_plan_is_zero() {
        let plan = SegmentPlan { segments: Vec::new() };
        assert_eq!(plan.index_at(5_000), 0);
        assert_eq!(plan.start_of(0), 0);
    }

    #[test]
    fn from_index_keeps_the_tail_and_its_timeline_starts() {
        let plan = plan_fixed_segments(20_000, 4_000);
        let tail = plan.from_index(2);
        assert_eq!(tail.segments.len(), plan.segments.len() - 2);
        assert_eq!(tail.segments[0].start_ms, 8_000);
        assert_eq!(plan.start_of(2), 8_000);
    }

    #[test]
    fn from_index_past_the_end_is_empty_rather_than_a_panic() {
        let plan = plan_fixed_segments(10_000, 4_000);
        assert!(plan.from_index(99).segments.is_empty());
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn cuts_cover_the_run_and_each_is_the_first_keyframe_past_its_mark(
            keyframes_ms in prop::collection::btree_set(0u64..30_000, 0..20),
            within_ms in 0u64..1_000,
            from_us in 0u64..30_000_000,
            duration in 1u64..30_000,
            target in 1u64..8_000,
        ) {
            let keyframes: Vec<u64> = keyframes_ms.iter().map(|ms| ms * 1_000 + within_ms).collect();
            let plan = plan_cuts(&keyframes, from_us, duration, target);
            let duration_us = duration * 1_000;
            if from_us >= duration_us {
                prop_assert!(plan.segments.is_empty());
                return Ok(());
            }
            let inside: Vec<u64> =
                keyframes.iter().copied().filter(|&k| k >= from_us && k < duration_us).collect();
            let anchor = inside.first().copied().unwrap_or(from_us);
            prop_assert_eq!(plan.segments[0].start_ms, from_us / 1_000);
            let mut acc = from_us / 1_000;
            let mut previous = from_us;
            for (i, segment) in plan.segments.iter().enumerate() {
                prop_assert!(segment.duration_ms > 0);
                prop_assert_eq!(segment.start_ms, acc);
                acc += segment.duration_ms;
                if i > 0 {
                    let cut = inside.iter().copied().find(|k| k / 1_000 == segment.start_ms);
                    prop_assert!(cut.is_some(), "segment {} starts on no keyframe", i);
                    let cut = cut.unwrap_or_default();
                    let mark = i as u64 * target * 1_000;
                    prop_assert!(cut - anchor >= mark);
                    prop_assert!(
                        !inside.iter().any(|&k| k > previous && k < cut && k - anchor >= mark)
                    );
                    previous = cut;
                }
            }
            prop_assert_eq!(acc, duration);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub start_ms: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentPlan {
    pub segments: Vec<Segment>,
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

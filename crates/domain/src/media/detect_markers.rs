use crate::media::{CreditsMarker, DetectedMarkers, FingerprintedVersion, IntroMarker};

pub fn detect_markers(episodes: &[FingerprintedVersion], frame_ms: u64) -> DetectedMarkers {
    if episodes.len() < 2 {
        return DetectedMarkers::default();
    }
    let a = &episodes[0].fingerprint.0;
    let b = &episodes[1].fingerprint.0;
    let intro_sig = head_signature(a, b);
    let credits_sig = tail_signature(a, b);

    let mut intros = Vec::new();
    let mut credits = Vec::new();
    for episode in episodes {
        let frames = &episode.fingerprint.0;
        if let Some((start, end)) = locate(frames, &intro_sig) {
            intros.push(IntroMarker {
                version: episode.version.clone(),
                start_ms: start * frame_ms,
                end_ms: end * frame_ms,
            });
        }
        if let Some((start, end)) = locate(frames, &credits_sig) {
            credits.push(CreditsMarker {
                version: episode.version.clone(),
                start_ms: start * frame_ms,
                end_ms: end * frame_ms,
            });
        }
    }
    DetectedMarkers { intros, credits }
}

fn head_signature(a: &[u32], b: &[u32]) -> Vec<u32> {
    let head_a = &a[..a.len() / 2];
    let head_b = &b[..b.len() / 2];
    signature(head_a, head_b)
}

fn tail_signature(a: &[u32], b: &[u32]) -> Vec<u32> {
    let tail_a = &a[a.len() / 2..];
    let tail_b = &b[b.len() / 2..];
    signature(tail_a, tail_b)
}

fn signature(a: &[u32], b: &[u32]) -> Vec<u32> {
    let (start, len) = longest_common_run(a, b);
    a[start..start + len].to_vec()
}

fn longest_common_run(a: &[u32], b: &[u32]) -> (usize, usize) {
    let mut best_start = 0;
    let mut best_len = 0;
    let mut prev = vec![0usize; b.len() + 1];
    for (i, &av) in a.iter().enumerate() {
        let mut curr = vec![0usize; b.len() + 1];
        for (j, &bv) in b.iter().enumerate() {
            if av == bv {
                let run = prev[j] + 1;
                curr[j + 1] = run;
                if run > best_len {
                    best_len = run;
                    best_start = i + 1 - run;
                }
            }
        }
        prev = curr;
    }
    (best_start, best_len)
}

fn locate(haystack: &[u32], needle: &[u32]) -> Option<(u64, u64)> {
    if needle.is_empty() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|idx| (idx as u64, (idx + needle.len()) as u64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::VersionId;
    use crate::media::Fingerprint;

    fn episode(id: &str, frames: &[u32]) -> FingerprintedVersion {
        FingerprintedVersion {
            version: VersionId(id.to_owned()),
            fingerprint: Fingerprint(frames.to_vec()),
        }
    }

    #[test]
    fn fewer_than_two_episodes_yields_nothing() {
        let markers = detect_markers(&[episode("v1", &[1, 2, 3, 4])], 100);
        assert!(markers.intros.is_empty());
        assert!(markers.credits.is_empty());
    }

    #[test]
    fn shared_head_and_tail_runs_become_markers() {
        let e1 = episode("v1", &[7, 7, 7, 10, 20, 9, 9, 9]);
        let e2 = episode("v2", &[7, 7, 7, 30, 40, 9, 9, 9]);
        let markers = detect_markers(&[e1, e2], 1000);

        assert_eq!(markers.intros.len(), 2);
        assert_eq!(markers.intros[0].version, VersionId("v1".to_owned()));
        assert_eq!(markers.intros[0].start_ms, 0);
        assert_eq!(markers.intros[0].end_ms, 3000);

        assert_eq!(markers.credits.len(), 2);
        assert_eq!(markers.credits[1].version, VersionId("v2".to_owned()));
        assert_eq!(markers.credits[1].start_ms, 5000);
        assert_eq!(markers.credits[1].end_ms, 8000);
    }

    #[test]
    fn episode_missing_the_signature_is_skipped() {
        let e1 = episode("v1", &[7, 7, 7, 10, 20, 9, 9, 9]);
        let e2 = episode("v2", &[7, 7, 7, 30, 40, 9, 9, 9]);
        let e3 = episode("v3", &[1, 2, 3, 4, 5, 6, 7, 8]);
        let markers = detect_markers(&[e1, e2, e3], 1000);

        assert_eq!(markers.intros.len(), 2);
        assert_eq!(markers.credits.len(), 2);
    }

    #[test]
    fn no_shared_run_yields_nothing() {
        let e1 = episode("v1", &[1, 2, 3, 4]);
        let e2 = episode("v2", &[5, 6, 7, 8]);
        let markers = detect_markers(&[e1, e2], 100);
        assert!(markers.intros.is_empty());
        assert!(markers.credits.is_empty());
    }

    #[test]
    fn empty_fingerprints_yield_nothing() {
        let markers = detect_markers(&[episode("v1", &[]), episode("v2", &[])], 100);
        assert!(markers.intros.is_empty());
        assert!(markers.credits.is_empty());
    }
}

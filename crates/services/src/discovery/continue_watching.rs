use domain::playback::PlaybackProgress;

pub fn continue_watching(progress: &[PlaybackProgress]) -> Vec<PlaybackProgress> {
    let mut items: Vec<PlaybackProgress> = progress
        .iter()
        .filter(|p| p.position_ms > 0)
        .cloned()
        .collect();
    items.sort_by_key(|p| std::cmp::Reverse(p.updated_at));
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::VersionId;
    use domain::user::UserId;
    use jiff::{SignedDuration, Timestamp};

    fn progress(version: &str, position_ms: u64, seconds: i64) -> PlaybackProgress {
        PlaybackProgress {
            user: UserId("u1".to_owned()),
            version: VersionId(version.to_owned()),
            position_ms,
            updated_at: Timestamp::UNIX_EPOCH + SignedDuration::from_secs(seconds),
        }
    }

    fn versions(items: &[PlaybackProgress]) -> Vec<String> {
        items.iter().map(|p| p.version.0.clone()).collect()
    }

    #[test]
    fn orders_started_by_recency() {
        let entries = [
            progress("older", 50, 10),
            progress("newest", 50, 30),
            progress("middle", 50, 20),
        ];
        let rows = continue_watching(&entries);
        assert_eq!(versions(&rows), vec!["newest", "middle", "older"]);
    }

    #[test]
    fn excludes_unstarted() {
        let entries = [progress("unstarted", 0, 20), progress("resume", 120, 10)];
        let rows = continue_watching(&entries);
        assert_eq!(versions(&rows), vec!["resume"]);
    }
}

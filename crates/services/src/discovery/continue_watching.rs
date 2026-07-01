use std::collections::HashSet;

use domain::catalog::VersionId;
use domain::discovery::ContinueWatchingItem;
use domain::playback::PlaybackProgress;

pub fn continue_watching(
    progress: &[PlaybackProgress],
    completed: &HashSet<VersionId>,
) -> Vec<ContinueWatchingItem> {
    let mut items: Vec<ContinueWatchingItem> = progress
        .iter()
        .filter(|p| p.position_ms > 0 && !completed.contains(&p.version))
        .cloned()
        .map(|progress| ContinueWatchingItem { progress })
        .collect();
    items.sort_by_key(|item| std::cmp::Reverse(item.progress.updated_at));
    items
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn versions(items: &[ContinueWatchingItem]) -> Vec<String> {
        items.iter().map(|i| i.progress.version.0.clone()).collect()
    }

    #[test]
    fn orders_started_unfinished_by_recency() {
        let entries = [
            progress("older", 50, 10),
            progress("newest", 50, 30),
            progress("middle", 50, 20),
        ];
        let rows = continue_watching(&entries, &HashSet::new());
        assert_eq!(versions(&rows), vec!["newest", "middle", "older"]);
    }

    #[test]
    fn excludes_completed_and_unstarted() {
        let entries = [
            progress("done", 500, 30),
            progress("unstarted", 0, 20),
            progress("resume", 120, 10),
        ];
        let completed = HashSet::from([VersionId("done".to_owned())]);
        let rows = continue_watching(&entries, &completed);
        assert_eq!(versions(&rows), vec!["resume"]);
    }
}

use std::collections::HashSet;

use domain::discovery::ContinueWatchingItem;
use domain::playback::PlaybackProgress;
use domain::session::NowPlaying;

pub fn continue_watching(progress: &[PlaybackProgress]) -> Vec<PlaybackProgress> {
    let mut items: Vec<PlaybackProgress> = progress
        .iter()
        .filter(|p| p.position_ms > 0)
        .cloned()
        .collect();
    items.sort_by_key(|p| std::cmp::Reverse(p.updated_at));
    items
}

pub fn drop_resumable(
    now_playing: Vec<NowPlaying>,
    in_progress: &[ContinueWatchingItem],
) -> Vec<NowPlaying> {
    let resumable: HashSet<&str> = in_progress
        .iter()
        .map(|item| item.progress.version.0.as_str())
        .collect();
    now_playing
        .into_iter()
        .filter(|n| !resumable.contains(n.session.version.0.as_str()))
        .collect()
}

pub fn drop_unstarted(now_playing: Vec<NowPlaying>) -> Vec<NowPlaying> {
    now_playing
        .into_iter()
        .filter(|n| n.card.progress_percent > 0)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{MovieId, TitleId, VersionId};
    use domain::playback::ResumeCard;
    use domain::session::{
        DeliveryMode, PlaybackSession, PlaybackState, SelectedTracks, SessionId,
    };
    use domain::user::UserId;
    use jiff::{SignedDuration, Timestamp};

    fn progress(version: &str, position_ms: u64, seconds: i64) -> PlaybackProgress {
        PlaybackProgress {
            user: UserId("u1".to_owned()),
            version: VersionId(version.to_owned()),
            position_ms,
            audio_track: None,
            subtitle: None,
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

    fn card() -> ResumeCard {
        card_at(40)
    }

    fn card_at(progress_percent: u8) -> ResumeCard {
        ResumeCard {
            title: TitleId::Movie(MovieId("m1".to_owned())),
            display_title: "Big Buck Bunny".to_owned(),
            artwork: Vec::new(),
            duration_ms: 1000,
            progress_percent,
            year: None,
            series_title: None,
            series_artwork: Vec::new(),
            season_number: None,
            episode_number: None,
        }
    }

    fn item(version: &str) -> ContinueWatchingItem {
        ContinueWatchingItem {
            progress: progress(version, 400, 10),
            card: card(),
        }
    }

    fn playing(version: &str) -> NowPlaying {
        NowPlaying {
            session: PlaybackSession {
                id: SessionId(format!("s-{version}")),
                user: UserId("u1".to_owned()),
                device: None,
                version: VersionId(version.to_owned()),
                mode: DeliveryMode::Direct,
                position_ms: 580,
                state: PlaybackState::Playing,
                selected: SelectedTracks {
                    audio_track: None,
                    subtitle_track: None,
                    subtitle_delivery: None,
                },
                started_at: Timestamp::UNIX_EPOCH,
                last_heartbeat_at: Timestamp::UNIX_EPOCH,
                completed: false,
            },
            card: card(),
        }
    }

    #[test]
    fn a_version_with_stored_progress_is_not_also_reported_as_playing() {
        let in_progress = [item("v1")];
        let rows = drop_resumable(vec![playing("v1"), playing("v2")], &in_progress);
        assert_eq!(
            rows.iter()
                .map(|n| n.session.version.0.clone())
                .collect::<Vec<String>>(),
            vec!["v2"]
        );
    }

    #[test]
    fn a_session_with_no_stored_progress_yet_survives() {
        let rows = drop_resumable(vec![playing("v1")], &[]);
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn a_session_sitting_at_the_start_never_reaches_the_rail() {
        let stalled = NowPlaying {
            card: card_at(0),
            ..playing("v1")
        };
        let rows = drop_unstarted(vec![stalled, playing("v2")]);
        assert_eq!(
            rows.iter()
                .map(|n| n.session.version.0.clone())
                .collect::<Vec<String>>(),
            vec!["v2"],
            "a card at 0% has nothing to resume while the title page offers no resume either"
        );
    }

    #[test]
    fn several_stale_sessions_for_one_version_all_go() {
        let in_progress = [item("v1")];
        let rows = drop_resumable(
            vec![playing("v1"), playing("v1"), playing("v1")],
            &in_progress,
        );
        assert!(rows.is_empty());
    }
}

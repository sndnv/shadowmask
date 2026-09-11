use domain::catalog::{EpisodeId, MovieId, TitleId, VersionId};
use domain::common::PageRequest;
use domain::media::SubtitleFileId;
use domain::playback::{PlaybackProgress, SubtitleOverride, SubtitleTrackRef};
use domain::repository::ProgressRepository;
use domain::user::UserId;
use jiff::Timestamp;

fn ts(second: i64) -> Timestamp {
    Timestamp::from_second(second).expect("valid timestamp")
}

fn page(offset: u32, limit: u32) -> PageRequest {
    PageRequest { offset, limit }
}

fn movie(id: &str) -> TitleId {
    TitleId::Movie(MovieId(id.into()))
}

fn episode(id: &str) -> TitleId {
    TitleId::Episode(EpisodeId(id.into()))
}

fn progress(user: &str, version: &str, position_ms: u64, updated_at: i64) -> PlaybackProgress {
    PlaybackProgress {
        user: UserId(user.into()),
        version: VersionId(version.into()),
        position_ms,
        audio_track: None,
        subtitle: None,
        updated_at: ts(updated_at),
    }
}

pub async fn progress_repository_contract<R: ProgressRepository>(repo: R) {
    let u1 = UserId("u1".into());
    let u2 = UserId("u2".into());
    let v1 = VersionId("v1".into());

    assert!(repo.get(&u1, &v1).await.unwrap().is_none());
    assert!(repo.list_in_progress(&u1).await.unwrap().is_empty());
    let empty = repo.history(&u1, page(0, 10)).await.unwrap();
    assert_eq!(empty.total, 0);
    assert!(empty.items.is_empty());

    repo.upsert(progress("u1", "v1", 1000, 10)).await.unwrap();
    let got = repo.get(&u1, &v1).await.unwrap().unwrap();
    assert_eq!(got.position_ms, 1000);
    assert_eq!(got.user, u1);
    assert_eq!(got.updated_at, ts(10));

    repo.upsert(progress("u1", "v1", 2500, 20)).await.unwrap();
    let got = repo.get(&u1, &v1).await.unwrap().unwrap();
    assert_eq!(got.position_ms, 2500);
    assert_eq!(repo.list_in_progress(&u1).await.unwrap().len(), 1);

    repo.upsert(PlaybackProgress {
        audio_track: Some(2),
        subtitle: Some(SubtitleOverride::Track(SubtitleTrackRef::Embedded(4))),
        ..progress("u1", "v1", 2500, 20)
    })
    .await
    .unwrap();
    let got = repo.get(&u1, &v1).await.unwrap().unwrap();
    assert_eq!(got.audio_track, Some(2));
    assert_eq!(got.subtitle, Some(SubtitleOverride::Track(SubtitleTrackRef::Embedded(4))));
    assert!(got.has_override());

    repo.upsert(PlaybackProgress {
        subtitle: Some(SubtitleOverride::Track(SubtitleTrackRef::File(SubtitleFileId(
            "sf1".into(),
        )))),
        ..progress("u1", "v1", 2500, 20)
    })
    .await
    .unwrap();
    let got = repo.get(&u1, &v1).await.unwrap().unwrap();
    assert_eq!(got.audio_track, None);
    assert_eq!(
        got.subtitle,
        Some(SubtitleOverride::Track(SubtitleTrackRef::File(SubtitleFileId("sf1".into()))))
    );

    // Off is a stored choice, distinct from having no override at all.
    repo.upsert(PlaybackProgress {
        subtitle: Some(SubtitleOverride::Off),
        ..progress("u1", "v1", 2500, 20)
    })
    .await
    .unwrap();
    let got = repo.get(&u1, &v1).await.unwrap().unwrap();
    assert_eq!(got.subtitle, Some(SubtitleOverride::Off));
    assert!(got.has_override());

    repo.upsert(progress("u1", "v1", 2500, 20)).await.unwrap();
    let got = repo.get(&u1, &v1).await.unwrap().unwrap();
    assert!(!got.has_override());
    assert_eq!(got.subtitle, None);

    repo.upsert(progress("u1", "v2", 500, 30)).await.unwrap();
    repo.upsert(progress("u1", "v3", 500, 30)).await.unwrap();
    let in_progress = repo.list_in_progress(&u1).await.unwrap();
    let versions: Vec<_> = in_progress.iter().map(|p| p.version.0.as_str()).collect();
    assert_eq!(versions, ["v2", "v3", "v1"]);

    repo.record_view(&u1, &movie("m1"), ts(100)).await.unwrap();
    repo.record_view(&u1, &episode("e1"), ts(200)).await.unwrap();
    repo.record_view(&u1, &movie("m2"), ts(50)).await.unwrap();
    let listed = repo.history(&u1, page(0, 10)).await.unwrap();
    assert_eq!(listed.total, 3);
    let titles: Vec<_> = listed.items.iter().map(|h| h.title.id().to_owned()).collect();
    assert_eq!(titles, ["e1", "m1", "m2"]);
    let first = &listed.items[0];
    assert_eq!(first.play_count, 1);
    assert!(first.completed);
    assert!(first.watched);
    assert_eq!(first.last_watched_at, Some(ts(200)));
    assert_eq!(first.user, u1);

    // Two titles watched in the same instant still have to come back in a fixed order, or the
    // history page reshuffles itself between reloads.
    repo.record_view(&u1, &movie("m2"), ts(200)).await.unwrap();
    let tied = repo.history(&u1, page(0, 10)).await.unwrap();
    let tied_titles: Vec<_> = tied.items.iter().map(|h| h.title.id().to_owned()).collect();
    assert_eq!(tied_titles, ["e1", "m2", "m1"]);

    repo.record_view(&u1, &movie("m1"), ts(300)).await.unwrap();
    repo.record_view(&u1, &movie("m1"), ts(400)).await.unwrap();
    let counted = repo.history(&u1, page(0, 10)).await.unwrap();
    assert_eq!(counted.total, 3);
    let m1 = counted.items.iter().find(|h| h.title.id() == "m1").unwrap();
    assert_eq!(m1.play_count, 3);
    assert_eq!(m1.last_watched_at, Some(ts(400)));

    repo.set_watched_flags(&u1, &movie("m1"), false).await.unwrap();
    let unmarked = repo.history(&u1, page(0, 10)).await.unwrap();
    assert_eq!(unmarked.total, 3);
    let m1 = unmarked.items.iter().find(|h| h.title.id() == "m1").unwrap();
    assert_eq!(m1.play_count, 3);
    assert_eq!(m1.last_watched_at, Some(ts(400)));
    assert!(!m1.watched);

    repo.set_watched_flags(&u1, &movie("m3"), true).await.unwrap();
    assert_eq!(repo.history(&u1, page(0, 10)).await.unwrap().total, 3);
    repo.set_watched_flags(&u1, &movie("m3"), false).await.unwrap();
    assert_eq!(repo.history(&u1, page(0, 10)).await.unwrap().total, 3);

    let paged = repo.history(&u1, page(1, 1)).await.unwrap();
    assert_eq!(paged.total, 3);
    assert_eq!(paged.items.len(), 1);
    assert_eq!(paged.offset, 1);
    assert_eq!(paged.limit, 1);
    let beyond = repo.history(&u1, page(10, 10)).await.unwrap();
    assert_eq!(beyond.total, 3);
    assert!(beyond.items.is_empty());

    assert!(repo.get(&u2, &v1).await.unwrap().is_none());
    assert!(repo.list_in_progress(&u2).await.unwrap().is_empty());
    assert_eq!(repo.history(&u2, page(0, 10)).await.unwrap().total, 0);
    repo.upsert(progress("u2", "v1", 42, 5)).await.unwrap();
    assert_eq!(repo.get(&u2, &v1).await.unwrap().unwrap().position_ms, 42);
    assert_eq!(repo.get(&u1, &v1).await.unwrap().unwrap().position_ms, 2500);

    repo.delete(&u1, &v1).await.unwrap();
    assert!(repo.get(&u1, &v1).await.unwrap().is_none());
    let remaining: Vec<_> =
        repo.list_in_progress(&u1).await.unwrap().into_iter().map(|p| p.version.0).collect();
    assert_eq!(remaining, ["v2", "v3"]);
    assert_eq!(repo.get(&u2, &v1).await.unwrap().unwrap().position_ms, 42);
    repo.delete(&u1, &v1).await.unwrap();

    repo.delete_history(&u1, "m2").await.unwrap();
    let after = repo.history(&u1, page(0, 10)).await.unwrap();
    assert_eq!(after.total, 2);
    let titles: Vec<_> = after.items.iter().map(|h| h.title.id().to_owned()).collect();
    assert_eq!(titles, ["m1", "e1"]);
    repo.delete_history(&u1, "m2").await.unwrap();
    assert_eq!(repo.history(&u1, page(0, 10)).await.unwrap().total, 2);

    repo.set_watched_flags(&u1, &episode("e1"), true).await.unwrap();
    repo.clear_history(&u1).await.unwrap();
    assert_eq!(repo.history(&u1, page(0, 10)).await.unwrap().total, 0);
    let states = repo.watched_state(&u1).await.unwrap();
    assert!(states.iter().any(|h| h.title.id() == "e1" && h.watched));
    assert!(!states.iter().any(|h| h.title.id() == "m1"));
}

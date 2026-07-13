use domain::catalog::{EpisodeId, MovieId, TitleId, VersionId};
use domain::common::PageRequest;
use domain::playback::{PlaybackProgress, WatchHistory};
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
        updated_at: ts(updated_at),
    }
}

fn history(
    user: &str,
    title: TitleId,
    watched: bool,
    play_count: u32,
    last_watched_at: Option<i64>,
    completed: bool,
) -> WatchHistory {
    WatchHistory {
        user: UserId(user.into()),
        title,
        watched,
        play_count,
        last_watched_at: last_watched_at.map(ts),
        completed,
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

    repo.upsert(progress("u1", "v2", 500, 30)).await.unwrap();
    repo.upsert(progress("u1", "v3", 500, 30)).await.unwrap();
    let in_progress = repo.list_in_progress(&u1).await.unwrap();
    let versions: Vec<_> = in_progress.iter().map(|p| p.version.0.as_str()).collect();
    assert_eq!(versions, ["v2", "v3", "v1"]);

    repo.record_history(history("u1", movie("m1"), true, 1, Some(100), false))
        .await
        .unwrap();
    repo.record_history(history("u1", episode("e1"), true, 2, Some(200), true))
        .await
        .unwrap();
    repo.record_history(history("u1", movie("m2"), false, 1, None, false))
        .await
        .unwrap();
    let listed = repo.history(&u1, page(0, 10)).await.unwrap();
    assert_eq!(listed.total, 3);
    let titles: Vec<_> = listed
        .items
        .iter()
        .map(|h| h.title.id().to_owned())
        .collect();
    assert_eq!(titles, ["e1", "m1", "m2"]);
    let first = &listed.items[0];
    assert_eq!(first.play_count, 2);
    assert!(first.completed);
    assert_eq!(first.last_watched_at, Some(ts(200)));
    assert_eq!(first.user, u1);
    let last = &listed.items[2];
    assert_eq!(last.last_watched_at, None);
    assert!(!last.watched);

    repo.record_history(history("u1", movie("m1"), true, 5, Some(300), true))
        .await
        .unwrap();
    let replaced = repo.history(&u1, page(0, 10)).await.unwrap();
    assert_eq!(replaced.total, 3);
    let m1 = replaced
        .items
        .iter()
        .find(|h| h.title.id() == "m1")
        .unwrap();
    assert_eq!(m1.play_count, 5);
    assert!(m1.completed);

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
    let remaining: Vec<_> = repo
        .list_in_progress(&u1)
        .await
        .unwrap()
        .into_iter()
        .map(|p| p.version.0)
        .collect();
    assert_eq!(remaining, ["v2", "v3"]);
    assert_eq!(repo.get(&u2, &v1).await.unwrap().unwrap().position_ms, 42);
    repo.delete(&u1, &v1).await.unwrap();
}

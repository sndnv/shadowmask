use domain::catalog::{EpisodeId, MovieId, TitleId, VersionId};
use domain::media::SubtitleFileId;
use domain::playback::{Favorite, SubtitleTrackRef, UserSubtitleOffset, WatchlistItem};
use domain::repository::PreferencesRepository;
use domain::user::UserId;
use jiff::Timestamp;

fn ts(second: i64) -> Timestamp {
    Timestamp::from_second(second).expect("valid timestamp")
}

fn movie(id: &str) -> TitleId {
    TitleId::Movie(MovieId(id.into()))
}

fn episode(id: &str) -> TitleId {
    TitleId::Episode(EpisodeId(id.into()))
}

fn watchlist(user: &str, title: TitleId, added_at: i64) -> WatchlistItem {
    WatchlistItem { user: UserId(user.into()), title, added_at: ts(added_at) }
}

fn favorite(user: &str, title: TitleId, added_at: i64) -> Favorite {
    Favorite { user: UserId(user.into()), title, added_at: ts(added_at) }
}

fn offset(
    user: &str,
    version: &str,
    subtitle: SubtitleTrackRef,
    offset_ms: i64,
) -> UserSubtitleOffset {
    UserSubtitleOffset {
        user: UserId(user.into()),
        version: VersionId(version.into()),
        subtitle,
        offset_ms,
    }
}

pub async fn preferences_repository_contract<R: PreferencesRepository>(repo: R) {
    let u1 = UserId("u1".into());
    let u2 = UserId("u2".into());
    let v1 = VersionId("v1".into());
    let embedded = SubtitleTrackRef::Embedded(2);
    let file = SubtitleTrackRef::File(SubtitleFileId("sub-1".into()));

    assert!(repo.list_watchlist(&u1).await.unwrap().is_empty());
    assert!(repo.list_favorites(&u1).await.unwrap().is_empty());
    assert!(repo.get_subtitle_offset(&u1, &v1, &embedded).await.unwrap().is_none());
    repo.remove_watchlist(&u1, "m1").await.unwrap();
    repo.remove_favorite(&u1, "m1").await.unwrap();

    repo.add_watchlist(watchlist("u1", movie("m2"), 20)).await.unwrap();
    repo.add_watchlist(watchlist("u1", episode("e1"), 10)).await.unwrap();
    repo.add_watchlist(watchlist("u1", movie("m1"), 10)).await.unwrap();
    let wl = repo.list_watchlist(&u1).await.unwrap();
    let ids: Vec<_> = wl.iter().map(|i| i.title.id().to_owned()).collect();
    assert_eq!(ids, ["e1", "m1", "m2"]);
    assert_eq!(wl[0].user, u1);

    repo.add_watchlist(watchlist("u1", movie("m1"), 99)).await.unwrap();
    let wl = repo.list_watchlist(&u1).await.unwrap();
    assert_eq!(wl.len(), 3);
    let m1 = wl.iter().find(|i| i.title.id() == "m1").unwrap();
    assert_eq!(m1.added_at, ts(99));

    repo.remove_watchlist(&u1, "m1").await.unwrap();
    let ids: Vec<_> =
        repo.list_watchlist(&u1).await.unwrap().iter().map(|i| i.title.id().to_owned()).collect();
    assert_eq!(ids, ["e1", "m2"]);

    repo.add_favorite(favorite("u1", movie("m1"), 5)).await.unwrap();
    repo.add_favorite(favorite("u1", episode("e2"), 5)).await.unwrap();
    let favs = repo.list_favorites(&u1).await.unwrap();
    let ids: Vec<_> = favs.iter().map(|i| i.title.id().to_owned()).collect();
    assert_eq!(ids, ["e2", "m1"]);
    assert_eq!(favs[0].user, u1);
    repo.add_favorite(favorite("u1", movie("m1"), 7)).await.unwrap();
    assert_eq!(repo.list_favorites(&u1).await.unwrap().len(), 2);
    repo.remove_favorite(&u1, "m1").await.unwrap();
    let favs = repo.list_favorites(&u1).await.unwrap();
    assert_eq!(favs.len(), 1);
    assert_eq!(favs[0].title.id(), "e2");

    repo.set_subtitle_offset(offset("u1", "v1", embedded.clone(), -250)).await.unwrap();
    repo.set_subtitle_offset(offset("u1", "v1", file.clone(), 500)).await.unwrap();
    let got = repo.get_subtitle_offset(&u1, &v1, &embedded).await.unwrap().unwrap();
    assert_eq!(got.offset_ms, -250);
    assert_eq!(got.subtitle, embedded);
    assert_eq!(got.version, v1);
    assert_eq!(got.user, u1);
    let got_file = repo.get_subtitle_offset(&u1, &v1, &file).await.unwrap().unwrap();
    assert_eq!(got_file.offset_ms, 500);
    assert_eq!(got_file.subtitle, file);

    repo.set_subtitle_offset(offset("u1", "v1", embedded.clone(), 999)).await.unwrap();
    assert_eq!(
        repo.get_subtitle_offset(&u1, &v1, &embedded).await.unwrap().unwrap().offset_ms,
        999
    );
    assert!(
        repo.get_subtitle_offset(&u1, &VersionId("v2".into()), &embedded).await.unwrap().is_none()
    );

    assert!(repo.list_watchlist(&u2).await.unwrap().is_empty());
    assert!(repo.list_favorites(&u2).await.unwrap().is_empty());
    assert!(repo.get_subtitle_offset(&u2, &v1, &embedded).await.unwrap().is_none());
    repo.add_watchlist(watchlist("u2", movie("m9"), 1)).await.unwrap();
    assert_eq!(repo.list_watchlist(&u2).await.unwrap().len(), 1);
    assert_eq!(repo.list_watchlist(&u1).await.unwrap().len(), 2);
}

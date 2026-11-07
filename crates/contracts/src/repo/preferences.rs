use std::collections::HashSet;

use domain::catalog::{EpisodeId, MovieId, TitleId, VersionId};
use domain::media::SubtitleFileId;
use domain::playback::{
    Favorite, Playlist, PlaylistId, QueueEntryId, SubtitleTrackRef, UserSubtitleOffset,
    WatchlistItem,
};
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

    playlists_contract(&repo).await;
    queue_contract(&repo).await;
}

fn playlist(id: &str, name: &str, created: i64, updated: i64) -> Playlist {
    Playlist {
        id: PlaylistId(id.into()),
        name: name.into(),
        created_at: ts(created),
        updated_at: ts(updated),
    }
}

async fn names<R: PreferencesRepository>(repo: &R, user: &UserId) -> Vec<String> {
    repo.list_playlists(user).await.unwrap().into_iter().map(|p| p.name).collect()
}

async fn order<R: PreferencesRepository>(repo: &R, id: &PlaylistId) -> Vec<(u32, String)> {
    repo.list_playlist_items(&UserId("u1".into()), id)
        .await
        .unwrap()
        .into_iter()
        .map(|item| (item.position, item.title.id().to_owned()))
        .collect()
}

async fn updated<R: PreferencesRepository>(repo: &R, id: &PlaylistId) -> Timestamp {
    repo.get_playlist(&UserId("u1".into()), id).await.unwrap().unwrap().updated_at
}

fn positions(ids: &[&str]) -> Vec<(u32, String)> {
    (0..).zip(ids.iter().map(|id| (*id).to_owned())).collect()
}

async fn playlists_contract<R: PreferencesRepository>(repo: &R) {
    let u1 = UserId("u1".into());
    let u2 = UserId("u2".into());
    let a = PlaylistId("p-a".into());
    let b = PlaylistId("p-b".into());

    assert!(repo.list_playlists(&u1).await.unwrap().is_empty());
    assert!(repo.get_playlist(&u1, &a).await.unwrap().is_none());

    repo.save_playlist(&u1, playlist("p-b", "beta", 1, 1)).await.unwrap();
    repo.save_playlist(&u1, playlist("p-a", "Alpha", 2, 2)).await.unwrap();
    assert_eq!(names(repo, &u1).await, ["Alpha", "beta"]);

    repo.save_playlist(&u1, playlist("p-a", "Gamma", 99, 5)).await.unwrap();
    let renamed = repo.get_playlist(&u1, &a).await.unwrap().unwrap();
    assert_eq!(renamed, playlist("p-a", "Gamma", 2, 5));
    assert_eq!(names(repo, &u1).await, ["beta", "Gamma"]);

    let added =
        repo.add_playlist_items(&u1, &a, &[movie("m1"), episode("e1"), movie("m2")], ts(10)).await;
    assert_eq!(added.unwrap(), Some(3));
    let items = repo.list_playlist_items(&u1, &a).await.unwrap();
    assert_eq!(items[1].title, episode("e1"));
    assert_eq!(items[1].added_at, ts(10));
    assert_eq!(order(repo, &a).await, positions(&["m1", "e1", "m2"]));
    assert_eq!(updated(repo, &a).await, ts(10));

    assert_eq!(
        repo.add_playlist_items(&u1, &a, &[movie("m2"), movie("m3")], ts(11)).await.unwrap(),
        Some(1)
    );
    assert_eq!(order(repo, &a).await, positions(&["m1", "e1", "m2", "m3"]));
    assert_eq!(repo.add_playlist_items(&u1, &a, &[movie("m2")], ts(12)).await.unwrap(), Some(0));
    assert_eq!(updated(repo, &a).await, ts(11), "adding nothing leaves the playlist as it was");
    let ghost = PlaylistId("ghost".into());
    let orphan = repo.add_playlist_items(&u1, &ghost, &[movie("m1")], ts(12)).await.unwrap();
    assert_eq!(orphan, None, "a playlist that does not exist takes no items");
    assert!(repo.list_playlist_items(&u1, &ghost).await.unwrap().is_empty());

    assert!(repo.move_playlist_item(&u1, &a, "m3", 1, ts(13)).await.unwrap());
    assert_eq!(order(repo, &a).await, positions(&["m1", "m3", "e1", "m2"]));
    assert_eq!(updated(repo, &a).await, ts(13));
    assert!(repo.move_playlist_item(&u1, &a, "m1", 2, ts(13)).await.unwrap());
    assert_eq!(order(repo, &a).await, positions(&["m3", "e1", "m1", "m2"]));
    assert!(repo.move_playlist_item(&u1, &a, "e1", 99, ts(13)).await.unwrap());
    assert_eq!(order(repo, &a).await, positions(&["m3", "m1", "m2", "e1"]));
    assert!(!repo.move_playlist_item(&u1, &a, "ghost", 0, ts(13)).await.unwrap());

    repo.remove_playlist_item(&u1, &a, "m1", ts(14)).await.unwrap();
    assert_eq!(order(repo, &a).await, positions(&["m3", "m2", "e1"]));
    assert_eq!(updated(repo, &a).await, ts(14));
    repo.remove_playlist_item(&u1, &a, "ghost", ts(15)).await.unwrap();
    assert_eq!(updated(repo, &a).await, ts(14), "removing nothing leaves the playlist as it was");

    assert!(repo.list_playlist_items(&u1, &b).await.unwrap().is_empty());
    assert!(repo.list_playlists(&u2).await.unwrap().is_empty());
    assert!(repo.get_playlist(&u2, &a).await.unwrap().is_none());
    assert!(repo.list_playlist_items(&u2, &a).await.unwrap().is_empty());

    repo.save_playlist(&u2, playlist("p-e1", "\u{e9}a", 1, 1)).await.unwrap();
    repo.save_playlist(&u2, playlist("p-e2", "\u{c9}z", 1, 1)).await.unwrap();
    assert_eq!(
        names(repo, &u2).await,
        ["\u{c9}z", "\u{e9}a"],
        "only ASCII letters compare without case, so an accented capital sorts first"
    );
}

async fn queued<R: PreferencesRepository>(repo: &R) -> Vec<(QueueEntryId, String)> {
    repo.list_queue(&UserId("u1".into()))
        .await
        .unwrap()
        .into_iter()
        .map(|entry| (entry.id, entry.title.id().to_owned()))
        .collect()
}

fn titles_of(entries: &[(QueueEntryId, String)]) -> Vec<&str> {
    entries.iter().map(|(_, title)| title.as_str()).collect()
}

async fn queue_contract<R: PreferencesRepository>(repo: &R) {
    let u1 = UserId("u1".into());
    let u2 = UserId("u2".into());
    let a = PlaylistId("p-a".into());
    let b = PlaylistId("p-b".into());

    assert!(repo.list_queue(&u1).await.unwrap().is_empty());
    assert_eq!(repo.queue_playlist(&u1, &a, false).await.unwrap(), 3);
    assert_eq!(titles_of(&queued(repo).await), ["m3", "m2", "e1"]);

    assert_eq!(repo.queue_playlist(&u1, &a, true).await.unwrap(), 3);
    let both = queued(repo).await;
    assert_eq!(titles_of(&both[..3]), ["m3", "m2", "e1"], "a shuffle appends behind the queue");
    let mut shuffled = titles_of(&both[3..]);
    shuffled.sort_unstable();
    assert_eq!(shuffled, ["e1", "m2", "m3"]);
    let ids: HashSet<&QueueEntryId> = both.iter().map(|(id, _)| id).collect();
    assert_eq!(ids.len(), 6, "every entry has its own id");

    repo.remove_first_queued(&u1, &movie("m2")).await.unwrap();
    let after = queued(repo).await;
    assert_eq!(titles_of(&after[..2]), ["m3", "e1"]);
    assert_eq!(after.len(), 5);
    assert!(titles_of(&after[2..]).contains(&"m2"), "only the earliest entry leaves");
    repo.remove_first_queued(&u1, &movie("ghost")).await.unwrap();
    assert_eq!(queued(repo).await.len(), 5);

    repo.remove_queue_entry(&u1, &after[0].0).await.unwrap();
    assert_eq!(queued(repo).await, after[1..]);
    repo.remove_queue_entry(&u1, &QueueEntryId("not-a-number".into())).await.unwrap();
    repo.remove_queue_entry(&u1, &QueueEntryId("999999".into())).await.unwrap();
    assert_eq!(queued(repo).await.len(), 4);

    assert_eq!(repo.queue_playlist(&u1, &b, false).await.unwrap(), 0);
    assert!(repo.list_queue(&u2).await.unwrap().is_empty());

    repo.clear_queue(&u1).await.unwrap();
    assert!(repo.list_queue(&u1).await.unwrap().is_empty());
    repo.queue_playlist(&u1, &a, false).await.unwrap();
    assert!(queued(repo).await.iter().all(|(id, _)| !ids.contains(id)), "ids are never reused");

    assert!(repo.delete_playlist(&u1, &a).await.unwrap());
    assert!(repo.get_playlist(&u1, &a).await.unwrap().is_none());
    assert!(repo.list_playlist_items(&u1, &a).await.unwrap().is_empty());
    assert!(!repo.delete_playlist(&u1, &a).await.unwrap());
    let after = repo.add_playlist_items(&u1, &a, &[movie("m1")], ts(30)).await.unwrap();
    assert_eq!(after, None, "a deleted playlist takes no items");
    assert!(repo.list_playlist_items(&u1, &a).await.unwrap().is_empty());
    assert_eq!(names(repo, &u1).await, ["beta"]);
    assert_eq!(queued(repo).await.len(), 3, "the queue keeps what it copied");
}

use domain::common::{LanguageCode, PageRequest};
use domain::error::RepositoryError;
use domain::library::LibraryId;
use domain::metadata::ContentRating;
use domain::repository::UserRepository;
use domain::user::{Role, User, UserId};
use jiff::Timestamp;

fn ts(second: i64) -> Timestamp {
    Timestamp::from_second(second).expect("valid timestamp")
}

fn page(offset: u32, limit: u32) -> PageRequest {
    PageRequest { offset, limit }
}

fn user(id: &str, username: &str, role: Role, created_at: Timestamp) -> User {
    User {
        id: UserId(id.into()),
        username: username.into(),
        password_hash: format!("hash-{id}"),
        role,
        max_content_rating: Some(ContentRating {
            system: "MPAA".into(),
            code: "PG-13".into(),
        }),
        preferred_audio: vec![LanguageCode("en".into()), LanguageCode("de".into())],
        preferred_subtitle: Vec::new(),
        concurrent_stream_limit: Some(2),
        bitrate_cap: Some(8_000_000),
        created_at,
        updated_at: created_at,
    }
}

pub async fn user_repository_contract<R: UserRepository>(repo: R) {
    assert!(repo.get(&UserId("nope".into())).await.unwrap().is_none());
    assert!(repo.find_by_username("nobody").await.unwrap().is_none());
    let empty = repo.list(page(0, 10)).await.unwrap();
    assert_eq!(empty.total, 0);
    assert!(empty.items.is_empty());
    assert!(
        repo.list_library_access(&UserId("nope".into()))
            .await
            .unwrap()
            .is_empty()
    );

    repo.create(user("admin", "admin", Role::Admin, ts(1)))
        .await
        .unwrap();
    repo.create(user("u1", "alice", Role::User, ts(2)))
        .await
        .unwrap();

    let duplicate = repo.create(user("u2", "alice", Role::User, ts(3))).await;
    assert!(matches!(duplicate, Err(RepositoryError::Conflict(_))));

    let alice = repo.get(&UserId("u1".into())).await.unwrap().unwrap();
    assert_eq!(alice.username, "alice");
    assert_eq!(alice.role, Role::User);
    assert_eq!(
        alice.preferred_audio,
        vec![LanguageCode("en".into()), LanguageCode("de".into())]
    );
    assert_eq!(
        alice.max_content_rating,
        Some(ContentRating {
            system: "MPAA".into(),
            code: "PG-13".into(),
        })
    );
    assert_eq!(alice.concurrent_stream_limit, Some(2));
    assert_eq!(alice.bitrate_cap, Some(8_000_000));

    let by_name = repo.find_by_username("admin").await.unwrap().unwrap();
    assert_eq!(by_name.id, UserId("admin".into()));

    let listed = repo.list(page(0, 10)).await.unwrap();
    assert_eq!(listed.total, 2);
    assert_eq!(
        listed
            .items
            .iter()
            .map(|u| u.id.0.as_str())
            .collect::<Vec<_>>(),
        ["admin", "u1"]
    );
    let past_end = repo.list(page(5, 10)).await.unwrap();
    assert_eq!(past_end.total, 2);
    assert!(past_end.items.is_empty());
    assert_eq!(past_end.offset, 5);
    assert_eq!(past_end.limit, 10);

    let mut updated = alice.clone();
    updated.username = "alice2".into();
    updated.concurrent_stream_limit = None;
    updated.max_content_rating = None;
    updated.preferred_subtitle = vec![LanguageCode("fr".into())];
    updated.updated_at = ts(50);
    repo.update(updated).await.unwrap();
    let reloaded = repo.get(&UserId("u1".into())).await.unwrap().unwrap();
    assert_eq!(reloaded.username, "alice2");
    assert_eq!(reloaded.concurrent_stream_limit, None);
    assert_eq!(reloaded.max_content_rating, None);
    assert_eq!(reloaded.preferred_subtitle, vec![LanguageCode("fr".into())]);
    assert_eq!(reloaded.created_at, ts(2));
    assert_eq!(reloaded.updated_at, ts(50));

    repo.set_library_access(
        &UserId("u1".into()),
        &[LibraryId("lib1".into()), LibraryId("lib2".into())],
    )
    .await
    .unwrap();
    let access = repo
        .list_library_access(&UserId("u1".into()))
        .await
        .unwrap();
    assert_eq!(access.len(), 2);
    assert!(access.iter().all(|a| a.user == UserId("u1".into())));

    repo.set_library_access(&UserId("u1".into()), &[LibraryId("lib3".into())])
        .await
        .unwrap();
    let access = repo
        .list_library_access(&UserId("u1".into()))
        .await
        .unwrap();
    assert_eq!(access.len(), 1);
    assert_eq!(access[0].library, LibraryId("lib3".into()));

    repo.delete(&UserId("u1".into())).await.unwrap();
    assert!(repo.get(&UserId("u1".into())).await.unwrap().is_none());
    assert!(
        repo.list_library_access(&UserId("u1".into()))
            .await
            .unwrap()
            .is_empty()
    );
    let after_delete = repo.list(page(0, 10)).await.unwrap();
    assert_eq!(after_delete.total, 1);
}

use std::fs;

use domain::common::PageRequest;
use domain::repository::{ProgressRepository, UserRepository};
use domain::user::UserId;
use persistence::migrate::migrate_all;
use persistence::server::SqliteUserRepo;
use persistence::user::SqliteProgressRepo;

#[tokio::test]
async fn migrates_server_and_all_users() {
    let root = tempfile::tempdir().unwrap();
    let db_root = root.path();
    let users_dir = db_root.join("users");
    fs::create_dir_all(users_dir.join("alice")).unwrap();
    fs::create_dir_all(users_dir.join("bob")).unwrap();

    migrate_all(db_root).await.unwrap();
    migrate_all(db_root).await.unwrap();

    let server = db_root.join("server");
    for name in ["jobs.db", "libraries.db", "users.db", "catalog.db", "auth.db"] {
        assert!(server.join(name).exists(), "{name} should exist after migrate");
    }
    for who in ["alice", "bob"] {
        assert!(users_dir.join(who).join("progress.db").exists());
        assert!(users_dir.join(who).join("prefs.db").exists());
    }

    let users = SqliteUserRepo::connect(&server.join("users.db")).await.unwrap();
    let listed = users.list(PageRequest { offset: 0, limit: 10 }).await.unwrap();
    assert_eq!(listed.total, 0);

    let progress = SqliteProgressRepo::new(&users_dir);
    assert!(progress.list_in_progress(&UserId("alice".into())).await.unwrap().is_empty());
}

#[tokio::test]
async fn migrates_server_when_no_users_dir() {
    let root = tempfile::tempdir().unwrap();
    migrate_all(root.path()).await.unwrap();
    assert!(root.path().join("server").join("users.db").exists());
    assert!(!root.path().join("users").exists());
}

#[tokio::test]
async fn skips_non_directory_entries_in_users() {
    let root = tempfile::tempdir().unwrap();
    let users_dir = root.path().join("users");
    fs::create_dir_all(users_dir.join("carol")).unwrap();
    fs::write(users_dir.join("notes.txt"), b"x").unwrap();
    migrate_all(root.path()).await.unwrap();
    assert!(users_dir.join("carol").join("progress.db").exists());
}

#[tokio::test]
async fn errors_when_users_path_is_a_file() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("users"), b"x").unwrap();
    assert!(migrate_all(root.path()).await.is_err());
}

#[tokio::test]
async fn errors_when_server_dir_cannot_be_created() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("not-a-dir");
    fs::write(&file, b"x").unwrap();
    assert!(migrate_all(&file.join("nested")).await.is_err());
}

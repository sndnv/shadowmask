use contracts::repo::auth_token_repository_contract;
use domain::repository::AuthTokenRepository;
use domain::user::{AuthSession, AuthSessionId, PendingLink, Role, UserId};
use jiff::Timestamp;
use persistence::server::SqliteAuthTokenRepo;

fn ts(second: i64) -> Timestamp {
    Timestamp::from_second(second).unwrap()
}

async fn repo() -> (tempfile::TempDir, SqliteAuthTokenRepo) {
    let dir = tempfile::tempdir().unwrap();
    let repo = SqliteAuthTokenRepo::connect(&dir.path().join("auth.db"))
        .await
        .unwrap();
    (dir, repo)
}

fn session(jti: &str, user: &str) -> AuthSession {
    AuthSession {
        id: AuthSessionId(jti.into()),
        user: UserId(user.into()),
        refresh_token_hash: format!("hash-{jti}"),
        issued_at: ts(1_700_000_000),
        expires_at: ts(1_700_100_000),
    }
}

#[tokio::test]
async fn refresh_store_find_revoke() {
    let (_dir, repo) = repo().await;
    assert!(
        repo.find_refresh(&AuthSessionId("j1".into()))
            .await
            .unwrap()
            .is_none()
    );

    repo.store_refresh(session("j1", "u1")).await.unwrap();
    let found = repo
        .find_refresh(&AuthSessionId("j1".into()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.user, UserId("u1".into()));
    assert_eq!(found.refresh_token_hash, "hash-j1");
    assert_eq!(found.expires_at, ts(1_700_100_000));

    repo.revoke_refresh(&AuthSessionId("j1".into()))
        .await
        .unwrap();
    assert!(
        repo.find_refresh(&AuthSessionId("j1".into()))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn revoke_all_for_user_clears_only_that_user() {
    let (_dir, repo) = repo().await;
    repo.store_refresh(session("j1", "u1")).await.unwrap();
    repo.store_refresh(session("j2", "u1")).await.unwrap();
    repo.store_refresh(session("j3", "u2")).await.unwrap();

    repo.revoke_all_for_user(&UserId("u1".into()))
        .await
        .unwrap();

    assert!(
        repo.find_refresh(&AuthSessionId("j1".into()))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        repo.find_refresh(&AuthSessionId("j2".into()))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        repo.find_refresh(&AuthSessionId("j3".into()))
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn link_code_redeemed_once_and_respects_expiry() {
    let (_dir, repo) = repo().await;
    repo.store_link_code(PendingLink {
        code: "ABCD".into(),
        user: UserId("u1".into()),
        role: Role::Player,
        expires_at: ts(1_700_100_000),
    })
    .await
    .unwrap();

    let redeemed = repo
        .redeem_link_code("ABCD", ts(1_700_000_000))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(redeemed.user, UserId("u1".into()));
    assert_eq!(redeemed.role, Role::Player);

    assert!(
        repo.redeem_link_code("ABCD", ts(1_700_000_000))
            .await
            .unwrap()
            .is_none()
    );

    repo.store_link_code(PendingLink {
        code: "EXP".into(),
        user: UserId("u1".into()),
        role: Role::Player,
        expires_at: ts(1_700_000_000),
    })
    .await
    .unwrap();
    assert!(
        repo.redeem_link_code("EXP", ts(1_700_050_000))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn device_and_api_token_contract_holds_for_sqlite() {
    let (_dir, repo) = repo().await;
    auth_token_repository_contract(repo).await;
}

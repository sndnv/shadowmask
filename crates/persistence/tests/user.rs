use contracts::repo::user_repository_contract;
use persistence::server::SqliteUserRepo;

#[tokio::test]
async fn user_repository_contract_holds_for_sqlite() {
    let dir = tempfile::tempdir().unwrap();
    let repo = SqliteUserRepo::connect(&dir.path().join("users.db")).await.unwrap();
    user_repository_contract(repo).await;
}

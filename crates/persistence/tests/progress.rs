use contracts::repo::progress_repository_contract;
use persistence::user::SqliteProgressRepo;

#[tokio::test]
async fn progress_repository_contract_holds_for_sqlite() {
    let dir = tempfile::tempdir().unwrap();
    let repo = SqliteProgressRepo::new(dir.path());
    progress_repository_contract(repo).await;
}

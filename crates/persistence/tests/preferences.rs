use contracts::repo::preferences_repository_contract;
use persistence::user::SqlitePreferencesRepo;

#[tokio::test]
async fn preferences_repository_contract_holds_for_sqlite() {
    let dir = tempfile::tempdir().unwrap();
    let repo = SqlitePreferencesRepo::new(dir.path());
    preferences_repository_contract(repo).await;
}

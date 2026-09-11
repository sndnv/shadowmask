use contracts::repo::job_repository_contract;
use persistence::server::SqliteJobRepo;

#[tokio::test]
async fn job_repository_contract_holds_for_sqlite() {
    let dir = tempfile::tempdir().unwrap();
    let repo = SqliteJobRepo::connect(&dir.path().join("jobs.db")).await.unwrap();
    job_repository_contract(repo).await;
}

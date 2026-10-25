use contracts::repo::report_repository_contract;
use persistence::server::SqliteReportRepo;

#[tokio::test]
async fn report_repository_contract_holds_for_sqlite() {
    let dir = tempfile::tempdir().unwrap();
    let repo = SqliteReportRepo::connect(&dir.path().join("reports.db")).await.unwrap();
    report_repository_contract(repo).await;
}

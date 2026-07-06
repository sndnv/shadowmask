use contracts::repo::{job_repository_contract, library_repository_contract};
use services::mock::{MockJobStore, MockLibraryRepo};

#[tokio::test]
async fn job_repository_contract_holds_for_mock() {
    job_repository_contract(MockJobStore::new()).await;
}

#[tokio::test]
async fn library_repository_contract_holds_for_mock() {
    library_repository_contract(MockLibraryRepo::new(), async |repo, library| {
        repo.insert_library(library);
    })
    .await;
}

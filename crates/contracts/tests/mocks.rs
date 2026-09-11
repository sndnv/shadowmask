use contracts::repo::{
    auth_token_repository_contract, catalog_repository_contract, catalog_seed,
    job_repository_contract, library_repository_contract, preferences_repository_contract,
    progress_repository_contract, user_repository_contract,
};
use domain::repository::CatalogRepository;
use mocks::{
    MockAuthTokenRepo, MockCatalogRepo, MockJobStore, MockLibraryRepo, MockPreferencesRepo,
    MockProgressRepo, MockUserRepo,
};

#[tokio::test]
async fn job_repository_contract_holds_for_mock() {
    job_repository_contract(MockJobStore::new()).await;
}

#[tokio::test]
async fn auth_token_repository_contract_holds_for_mock() {
    auth_token_repository_contract(MockAuthTokenRepo::new()).await;
}

#[tokio::test]
async fn user_repository_contract_holds_for_mock() {
    user_repository_contract(MockUserRepo::new()).await;
}

#[tokio::test]
async fn progress_repository_contract_holds_for_mock() {
    progress_repository_contract(MockProgressRepo::new()).await;
}

#[tokio::test]
async fn preferences_repository_contract_holds_for_mock() {
    preferences_repository_contract(MockPreferencesRepo::new()).await;
}

#[tokio::test]
async fn library_repository_contract_holds_for_mock() {
    library_repository_contract(MockLibraryRepo::new()).await;
}

// The whole point of the mock is that a test passes unchanged against SQLite, which only
// holds if both are held to the same contract.
#[tokio::test]
async fn catalog_repository_contract_holds_for_mock() {
    catalog_repository_contract(MockCatalogRepo::new(), async |repo: &MockCatalogRepo| {
        let seed = catalog_seed();
        for movie in seed.movies {
            repo.add_movie(movie);
        }
        for series in seed.series {
            repo.add_series(series);
        }
        for season in seed.seasons {
            repo.add_season(season);
        }
        for episode in seed.episodes {
            repo.add_episode(episode);
        }
        for collection in seed.collections {
            repo.add_collection(collection);
        }
        for version in seed.versions {
            repo.add_version(version);
        }
        repo.seed_version_detail(seed.detail);
        for person in seed.people {
            repo.add_person(person);
        }
        for (owner, enrichment) in &seed.enrichment {
            repo.set_title_enrichment(owner, enrichment).await.unwrap();
        }
    })
    .await;
}

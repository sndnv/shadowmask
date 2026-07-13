use contracts::repo::{catalog_repository_contract, catalog_seed};
use domain::repository::CatalogRepository;
use persistence::server::SqliteCatalogRepo;

#[tokio::test]
async fn catalog_repository_contract_holds_for_sqlite() {
    let dir = tempfile::tempdir().unwrap();
    let repo = SqliteCatalogRepo::connect(&dir.path().join("catalog.db"))
        .await
        .unwrap();
    catalog_repository_contract(repo, async |repo: &SqliteCatalogRepo| {
        let seed = catalog_seed();
        for movie in seed.movies {
            repo.insert_movie(movie).await.unwrap();
        }
        for series in seed.series {
            repo.insert_series(series).await.unwrap();
        }
        for season in seed.seasons {
            repo.insert_season(season).await.unwrap();
        }
        for episode in seed.episodes {
            repo.insert_episode(episode).await.unwrap();
        }
        for collection in seed.collections {
            repo.insert_collection(collection).await.unwrap();
        }
        for version in seed.versions {
            repo.insert_version(version).await.unwrap();
        }
        repo.insert_version_detail(seed.detail).await.unwrap();
        for person in seed.people {
            repo.upsert_person(person).await.unwrap();
        }
        for (owner, enrichment) in &seed.enrichment {
            repo.set_title_enrichment(owner, enrichment).await.unwrap();
        }
    })
    .await;
}

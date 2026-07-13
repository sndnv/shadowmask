use contracts::repo::{search_index_contract, search_seed};
use domain::repository::CatalogRepository;
use persistence::server::SqliteCatalogRepo;

#[tokio::test]
async fn search_index_contract_holds_for_sqlite() {
    let dir = tempfile::tempdir().unwrap();
    let repo = SqliteCatalogRepo::connect(&dir.path().join("catalog.db"))
        .await
        .unwrap();
    search_index_contract(repo, async |repo: &SqliteCatalogRepo| {
        let seed = search_seed();
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
        for person in seed.people {
            repo.upsert_person(person).await.unwrap();
        }
    })
    .await;
}

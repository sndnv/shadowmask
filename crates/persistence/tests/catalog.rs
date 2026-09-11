use std::collections::HashSet;

use contracts::repo::{catalog_repository_contract, catalog_seed};
use domain::catalog::{Movie, MovieId, RandomScope, TitleId, TitleListFilter, Version, VersionId};
use domain::common::Quality;
use domain::library::LibraryId;
use domain::repository::CatalogRepository;
use jiff::Timestamp;
use persistence::server::SqliteCatalogRepo;

#[tokio::test]
async fn catalog_repository_contract_holds_for_sqlite() {
    let dir = tempfile::tempdir().unwrap();
    let repo = SqliteCatalogRepo::connect(&dir.path().join("catalog.db")).await.unwrap();
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

fn plain_movie(id: &str) -> Movie {
    Movie {
        id: MovieId(id.to_owned()),
        title: id.to_owned(),
        sort_title: id.to_owned(),
        year: None,
        overview: None,
        runtime_minutes: None,
        content_rating: None,
        manually_edited: false,
        added_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
        artwork: Vec::new(),
    }
}

fn movie_version(id: &str, movie: &str, available: bool) -> Version {
    Version {
        id: VersionId(id.to_owned()),
        title: TitleId::Movie(MovieId(movie.to_owned())),
        library: LibraryId("lib1".to_owned()),
        quality: Quality::Hd,
        container: "mkv".to_owned(),
        path: format!("/media/{id}.mkv"),
        size_bytes: 1,
        duration_ms: 1,
        available,
        added_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
    }
}

#[tokio::test]
async fn a_random_movie_is_drawn_from_the_whole_playable_pool() {
    let dir = tempfile::tempdir().unwrap();
    let repo = SqliteCatalogRepo::connect(&dir.path().join("catalog.db")).await.unwrap();

    for id in ["m1", "m2", "m3", "m4"] {
        repo.upsert_movie(plain_movie(id)).await.unwrap();
        repo.upsert_version(movie_version(&format!("v-{id}"), id, true)).await.unwrap();
    }
    repo.upsert_movie(plain_movie("offline")).await.unwrap();
    repo.upsert_version(movie_version("v-offline", "offline", false)).await.unwrap();
    repo.upsert_movie(plain_movie("fileless")).await.unwrap();

    let filter = TitleListFilter::default();
    let mut seen: HashSet<String> = HashSet::new();
    for _ in 0..200 {
        let picked = repo
            .random_playable_title(&RandomScope::Movies, &filter)
            .await
            .unwrap()
            .expect("four movies are playable");
        seen.insert(picked.id().to_owned());
    }

    assert_eq!(
        seen,
        ["m1", "m2", "m3", "m4"].iter().map(|id| (*id).to_owned()).collect::<HashSet<String>>(),
        "every playable movie should turn up, and nothing unplayable should"
    );
}

#[tokio::test]
async fn a_movie_whose_only_file_went_missing_is_never_picked() {
    let dir = tempfile::tempdir().unwrap();
    let repo = SqliteCatalogRepo::connect(&dir.path().join("catalog.db")).await.unwrap();

    repo.upsert_movie(plain_movie("gone")).await.unwrap();
    repo.upsert_version(movie_version("v-gone", "gone", false)).await.unwrap();

    assert!(
        repo.random_playable_title(&RandomScope::Movies, &TitleListFilter::default())
            .await
            .unwrap()
            .is_none()
    );
}

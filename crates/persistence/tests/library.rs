use contracts::repo::library_repository_contract;
use domain::catalog::{EpisodeId, MovieId, TitleId};
use domain::common::PageRequest;
use domain::library::{
    DuplicateCandidate, DuplicateCandidateId, LibraryId, MatchCandidate, UnmatchedFile,
    UnmatchedFileId,
};
use domain::repository::LibraryRepository;
use persistence::server::SqliteLibraryRepo;
use tempfile::TempDir;

async fn repo() -> (TempDir, SqliteLibraryRepo) {
    let dir = tempfile::tempdir().unwrap();
    let repo = SqliteLibraryRepo::connect(&dir.path().join("libraries.db"))
        .await
        .unwrap();
    (dir, repo)
}

fn page(offset: u32, limit: u32) -> PageRequest {
    PageRequest { offset, limit }
}

#[tokio::test]
async fn library_repository_contract_holds_for_sqlite() {
    let (_dir, repo) = repo().await;
    library_repository_contract(repo, async |repo: &SqliteLibraryRepo, library| {
        repo.insert_library(library).await.unwrap();
    })
    .await;
}

#[tokio::test]
async fn unmatched_round_trips_with_candidates_and_paginates() {
    let (_dir, repo) = repo().await;
    let library = LibraryId("lib1".into());
    for n in ["uf1", "uf2", "uf3"] {
        repo.insert_unmatched(UnmatchedFile {
            id: UnmatchedFileId(n.into()),
            library: library.clone(),
            path: format!("/media/{n}.mkv"),
            candidates: vec![
                MatchCandidate {
                    title: TitleId::Movie(MovieId("m1".into())),
                    confidence: 0.9,
                    label: "Alpha".into(),
                },
                MatchCandidate {
                    title: TitleId::Episode(EpisodeId("e1".into())),
                    confidence: 0.4,
                    label: "Beta".into(),
                },
            ],
        })
        .await
        .unwrap();
    }

    let first = repo.list_unmatched(&library, page(0, 2)).await.unwrap();
    assert_eq!(first.total, 3);
    assert_eq!(first.items.len(), 2);
    assert_eq!(first.items[0].id.0, "uf1");
    assert_eq!(first.items[0].candidates.len(), 2);
    assert_eq!(first.items[0].candidates[0].label, "Alpha");
    assert_eq!(
        first.items[0].candidates[1].title,
        TitleId::Episode(EpisodeId("e1".into()))
    );

    let past_end = repo.list_unmatched(&library, page(10, 2)).await.unwrap();
    assert_eq!(past_end.total, 3);
    assert!(past_end.items.is_empty());
    assert_eq!(past_end.offset, 10);
    assert_eq!(past_end.limit, 2);

    let other = repo
        .list_unmatched(&LibraryId("other".into()), page(0, 10))
        .await
        .unwrap();
    assert_eq!(other.total, 0);
}

#[tokio::test]
async fn duplicates_round_trip_with_paths_and_paginate() {
    let (_dir, repo) = repo().await;
    let library = LibraryId("lib1".into());
    for n in ["d1", "d2"] {
        repo.insert_duplicate(
            &library,
            DuplicateCandidate {
                id: DuplicateCandidateId(n.into()),
                title: TitleId::Movie(MovieId("m1".into())),
                paths: vec![format!("/a/{n}.mkv"), format!("/b/{n}.mkv")],
            },
        )
        .await
        .unwrap();
    }

    let all = repo.list_duplicates(&library, page(0, 10)).await.unwrap();
    assert_eq!(all.total, 2);
    assert_eq!(all.items.len(), 2);
    assert_eq!(all.items[0].id.0, "d1");
    assert_eq!(all.items[0].paths, vec!["/a/d1.mkv", "/b/d1.mkv"]);
    assert_eq!(all.items[0].title, TitleId::Movie(MovieId("m1".into())));

    let second = repo.list_duplicates(&library, page(1, 10)).await.unwrap();
    assert_eq!(second.total, 2);
    assert_eq!(second.items.len(), 1);
    assert_eq!(second.items[0].id.0, "d2");
}

use domain::catalog::{MovieId, TitleId};
use domain::common::PageRequest;
use domain::library::{
    DuplicateCandidate, DuplicateCandidateId, Library, LibraryId, LibraryKind, MatchCandidate,
    ResolutionStatus, ScanState, ScanStatus, UnmatchedFile, UnmatchedFileId, WatcherStrategy,
};
use domain::repository::LibraryRepository;
use jiff::Timestamp;

fn at(secs: i64) -> Timestamp {
    Timestamp::from_second(secs).expect("valid timestamp")
}

fn library(id: &str) -> Library {
    Library {
        id: LibraryId(id.into()),
        name: format!("Lib {id}"),
        kind: LibraryKind::Movie,
        roots: vec!["/media".into()],
        watcher: WatcherStrategy::Manual,
        scan_schedule: None,
        metadata_sources: Vec::new(),
        created_at: at(1_000),
        updated_at: at(1_000),
    }
}

fn page() -> PageRequest {
    PageRequest {
        offset: 0,
        limit: 10,
    }
}

pub async fn library_repository_contract<R: LibraryRepository>(repo: R) {
    let id = LibraryId("lib1".into());

    assert!(repo.list().await.unwrap().is_empty());
    assert!(repo.get(&id).await.unwrap().is_none());
    assert!(repo.scan_state(&id).await.unwrap().is_none());

    let empty = repo.list_unmatched(&id, page()).await.unwrap();
    assert_eq!(empty.total, 0);
    assert!(empty.items.is_empty());
    assert_eq!(empty.offset, 0);
    assert_eq!(empty.limit, 10);
    assert!(
        repo.list_duplicates(&id, page())
            .await
            .unwrap()
            .items
            .is_empty()
    );

    repo.upsert(library("lib1")).await.unwrap();
    assert_eq!(repo.list().await.unwrap().len(), 1);
    assert!(repo.get(&id).await.unwrap().is_some());
    assert!(repo.get(&LibraryId("nope".into())).await.unwrap().is_none());

    let mut renamed = library("lib1");
    renamed.name = "Renamed".to_owned();
    renamed.created_at = at(9_999);
    renamed.updated_at = at(2_000);
    repo.upsert(renamed).await.unwrap();
    let reloaded = repo.get(&id).await.unwrap().unwrap();
    assert_eq!(reloaded.name, "Renamed");
    assert_eq!(reloaded.created_at, at(1_000));
    assert_eq!(reloaded.updated_at, at(2_000));
    assert_eq!(repo.list().await.unwrap().len(), 1);

    let unmatched_id = UnmatchedFileId("uf1".into());
    let dup_id = DuplicateCandidateId("d1".into());
    let unmatched = |candidates| UnmatchedFile {
        id: unmatched_id.clone(),
        library: id.clone(),
        path: "/media/x.mkv".into(),
        candidates,
        created_at: at(100),
        updated_at: at(100),
    };
    let duplicate = || DuplicateCandidate {
        id: dup_id.clone(),
        title: TitleId::Movie(MovieId("m1".into())),
        paths: vec!["/a.mkv".into(), "/b.mkv".into()],
    };
    repo.insert_unmatched(unmatched(vec![MatchCandidate {
        title: TitleId::Movie(MovieId("m1".into())),
        confidence: 0.9,
        label: "Alpha".into(),
    }]))
    .await
    .unwrap();
    repo.insert_duplicate(&id, duplicate()).await.unwrap();
    assert_eq!(repo.list_unmatched(&id, page()).await.unwrap().total, 1);
    assert_eq!(repo.list_duplicates(&id, page()).await.unwrap().total, 1);

    let fetched = repo.get_unmatched(&unmatched_id).await.unwrap().unwrap();
    assert_eq!(fetched.path, "/media/x.mkv");
    assert_eq!(fetched.candidates.len(), 1);
    assert_eq!(fetched.candidates[0].label, "Alpha");
    assert!(
        repo.get_unmatched(&UnmatchedFileId("ghost".into()))
            .await
            .unwrap()
            .is_none()
    );

    repo.set_unmatched_status(&unmatched_id, ResolutionStatus::Resolved)
        .await
        .unwrap();
    repo.set_duplicate_status(&dup_id, ResolutionStatus::Dismissed)
        .await
        .unwrap();
    assert!(
        repo.list_unmatched(&id, page())
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        repo.list_duplicates(&id, page())
            .await
            .unwrap()
            .items
            .is_empty()
    );

    repo.insert_unmatched(unmatched(Vec::new())).await.unwrap();
    repo.insert_duplicate(&id, duplicate()).await.unwrap();
    assert!(
        repo.list_unmatched(&id, page())
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        repo.list_duplicates(&id, page())
            .await
            .unwrap()
            .items
            .is_empty()
    );

    repo.set_unmatched_status(&unmatched_id, ResolutionStatus::Active)
        .await
        .unwrap();
    repo.set_duplicate_status(&dup_id, ResolutionStatus::Active)
        .await
        .unwrap();
    assert_eq!(repo.list_unmatched(&id, page()).await.unwrap().total, 1);
    assert_eq!(repo.list_duplicates(&id, page()).await.unwrap().total, 1);

    repo.insert_unmatched(UnmatchedFile {
        id: unmatched_id.clone(),
        library: id.clone(),
        path: "/media/x.mkv".into(),
        candidates: Vec::new(),
        created_at: at(9_999),
        updated_at: at(200),
    })
    .await
    .unwrap();
    let reinserted = repo.get_unmatched(&unmatched_id).await.unwrap().unwrap();
    assert_eq!(reinserted.created_at, at(100));
    assert_eq!(reinserted.updated_at, at(200));

    let state = ScanState {
        library: id.clone(),
        status: ScanStatus::Running,
        progress: 0.5,
        started_at: Some(at(1_699_999_000)),
        last_scanned_at: Some(at(1_700_000_000)),
        error: None,
    };
    repo.save_scan_state(state).await.unwrap();
    let loaded = repo.scan_state(&id).await.unwrap().unwrap();
    assert_eq!(loaded.status, ScanStatus::Running);
    assert_eq!(loaded.started_at, Some(at(1_699_999_000)));
    assert_eq!(loaded.last_scanned_at, Some(at(1_700_000_000)));
    assert_eq!(loaded.progress, 0.5);

    repo.delete(&id).await.unwrap();
    assert!(repo.get(&id).await.unwrap().is_none());
    assert!(repo.list().await.unwrap().is_empty());
    assert!(repo.scan_state(&id).await.unwrap().is_none());
    assert!(
        repo.list_unmatched(&id, page())
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        repo.list_duplicates(&id, page())
            .await
            .unwrap()
            .items
            .is_empty()
    );
}

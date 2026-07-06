use domain::common::PageRequest;
use domain::library::{Library, LibraryId, LibraryKind, ScanState, ScanStatus, WatcherStrategy};
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
    }
}

fn page() -> PageRequest {
    PageRequest {
        offset: 0,
        limit: 10,
    }
}

pub async fn library_repository_contract<R: LibraryRepository>(
    repo: R,
    insert: impl AsyncFn(&R, Library),
) {
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

    insert(&repo, library("lib1")).await;
    assert_eq!(repo.list().await.unwrap().len(), 1);
    assert!(repo.get(&id).await.unwrap().is_some());
    assert!(repo.get(&LibraryId("nope".into())).await.unwrap().is_none());

    let state = ScanState {
        library: id.clone(),
        status: ScanStatus::Running,
        progress: 0.5,
        last_scanned_at: Some(at(1_700_000_000)),
        error: None,
    };
    repo.save_scan_state(state).await.unwrap();
    let loaded = repo.scan_state(&id).await.unwrap().unwrap();
    assert_eq!(loaded.status, ScanStatus::Running);
    assert_eq!(loaded.last_scanned_at, Some(at(1_700_000_000)));
    assert_eq!(loaded.progress, 0.5);
}

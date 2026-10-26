use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use jiff::Timestamp;
use serde_json::{Value, json};
use tower::ServiceExt;
use tracing_test::traced_test;

use api::{AppState, MaintenanceState, bounded, maintenance_router};
use contracts::Generator;
use domain::catalog::{Movie, MovieId, TitleId, Version, VersionId};
use domain::common::Quality;
use domain::job::{Job, JobCanceller, JobId, JobKind, JobPriority, JobStatus};
use domain::library::{
    Library, LibraryId, LibraryKind, LibraryOrigin, ScanState, ScanStatus, WatcherStrategy,
};
use domain::repository::{CatalogRepository, JobRepository, LibraryRepository};
use domain::user::{Role, User, UserId};
use mocks::{MockCatalogRepo, MockJobStore, MockLibraryRepo, MockUserDataStore, MockUserRepo};
use services::library::{MaintenanceServiceImpl, SubtitleJobPayload, TrickplayJobPayload};

const ADMIN: &str = "Bearer access:admin";
const USER: &str = "Bearer access:u1";
const PURGE: &str = "/api/v1/libraries/lib/purge";
const CLEAN: &str = "/api/v1/libraries/lib/clean";
const CLEAN_ALL: &str = "/api/v1/admin/clean";
const CANCEL: &str = "/api/v1/libraries/lib/jobs/cancel";

struct Signals;

impl JobCanceller for Signals {
    fn request_cancel(&self, _id: &JobId) -> bool {
        true
    }
}

struct Stores {
    libraries: MockLibraryRepo,
    users: MockUserRepo,
    jobs: MockJobStore,
    catalog: MockCatalogRepo,
    user_data: MockUserDataStore,
}

fn stores() -> Stores {
    let stores = Stores {
        libraries: MockLibraryRepo::new(),
        users: MockUserRepo::new(),
        jobs: MockJobStore::new(),
        catalog: MockCatalogRepo::new(),
        user_data: MockUserDataStore::new(),
    };
    stores.libraries.insert_library(Library {
        id: LibraryId("lib".into()),
        name: "Films".into(),
        origin: LibraryOrigin::Local,
        kind: LibraryKind::Movie,
        roots: vec!["/m".into()],
        sort_articles: Vec::new(),
        watcher: WatcherStrategy::Manual,
        scan_schedule: None,
        metadata_sources: Vec::new(),
        created_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
    });
    stores.users.insert(User {
        id: UserId("u1".into()),
        username: "u1".into(),
        password_hash: "hash".into(),
        role: Role::User,
        max_content_rating: None,
        preferred_audio: Vec::new(),
        preferred_subtitle: Vec::new(),
        concurrent_stream_limit: None,
        bitrate_cap: None,
        active: true,
        created_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
    });
    stores.catalog.add_movie(Movie {
        id: MovieId("m1".into()),
        title: "Alpha".into(),
        sort_title: "alpha".into(),
        year: None,
        overview: None,
        runtime_minutes: None,
        content_rating: None,
        manually_edited: false,
        added_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
        artwork: Vec::new(),
    });
    stores.catalog.add_version(Version {
        id: VersionId("v1".into()),
        title: TitleId::Movie(MovieId("m1".into())),
        library: LibraryId("lib".into()),
        quality: Quality::Hd,
        container: "mkv".into(),
        path: "/m/alpha.mkv".into(),
        size_bytes: 1,
        duration_ms: 1,
        available: true,
        added_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
    });
    stores
}

fn seed(stores: &Stores, id: &str, kind: JobKind, payload: String, status: JobStatus) {
    let mut job =
        Job::queued(JobId(id.into()), kind, JobPriority::Normal, payload, None, Timestamp::now());
    job.status = status;
    stores.jobs.seed(job);
}

fn app(stores: &Stores) -> Router {
    let generator = Generator::new();
    let state = AppState::new(
        generator.auth.clone(),
        generator.catalog.clone(),
        generator.session.clone(),
        generator.library.clone(),
        generator.user.clone(),
        generator.user_library.clone(),
        generator.discovery.clone(),
        generator.job.clone(),
    );
    let service = MaintenanceServiceImpl::new(
        stores.libraries.clone(),
        stores.users.clone(),
        stores.jobs.clone(),
        stores.catalog.clone(),
        stores.user_data.clone(),
        Arc::new(Signals),
    );
    bounded(maintenance_router(state, MaintenanceState::new(service)), Duration::from_secs(60))
}

async fn post(
    stores: &Stores,
    uri: &str,
    auth: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method("POST").uri(uri);
    if let Some(auth) = auth {
        builder = builder.header(header::AUTHORIZATION, auth);
    }
    let body = match body {
        Some(body) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    let response = app(stores).oneshot(builder.body(body).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[traced_test]
#[tokio::test]
async fn an_admin_purges_a_library_and_reads_what_went() {
    let stores = stores();

    let (status, answer) = post(&stores, PURGE, Some(ADMIN), None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        answer,
        json!({
            "movies": 1,
            "series": 0,
            "seasons": 0,
            "episodes": 0,
            "subtitles": 0,
            "wiped_users": 0
        })
    );
    assert!(stores.catalog.get_movie(&MovieId("m1".into())).await.unwrap().is_none());
    assert!(stores.user_data.forgotten().is_empty());
    assert!(logs_contain("User [admin] purged library [lib], user data wiped: [false]"));
}

#[tokio::test]
async fn user_data_is_wiped_when_the_body_asks_for_it() {
    let stores = stores();

    let wipe = Some(json!({ "wipe_user_data": true }));
    let (status, answer) = post(&stores, PURGE, Some(ADMIN), wipe).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(answer["wiped_users"], 1);
    assert_eq!(stores.user_data.forgotten().len(), 1);
}

#[tokio::test]
async fn a_busy_library_answers_409_naming_the_jobs_in_the_way() {
    let stores = stores();
    seed(&stores, "scan-1", JobKind::LibraryScan, "lib".into(), JobStatus::Queued);

    let (status, answer) = post(&stores, PURGE, Some(ADMIN), None).await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(answer["error"]["code"], "library_busy");
    let jobs = answer["jobs"].as_array().unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(
        (&jobs[0]["id"], &jobs[0]["kind"], &jobs[0]["status"]),
        (&json!("scan-1"), &json!("library_scan"), &json!("queued"))
    );
    assert!(stores.catalog.get_movie(&MovieId("m1".into())).await.unwrap().is_some());
}

fn with_missing_files(stores: &Stores) {
    for (version, movie, library) in [("v-gone", "m-gone", "lib"), ("v-old", "m-old", "deleted")] {
        stores.catalog.add_movie(Movie {
            id: MovieId(movie.into()),
            title: movie.into(),
            sort_title: movie.into(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        stores.catalog.add_version(Version {
            id: VersionId(version.into()),
            title: TitleId::Movie(MovieId(movie.into())),
            library: LibraryId(library.into()),
            quality: Quality::Hd,
            container: "mkv".into(),
            path: format!("/m/{version}.mkv"),
            size_bytes: 1,
            duration_ms: 1,
            available: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        });
    }
}

#[traced_test]
#[tokio::test]
async fn an_admin_cleans_a_library_and_reads_what_went() {
    let stores = stores();
    with_missing_files(&stores);

    let (status, answer) = post(&stores, CLEAN, Some(ADMIN), None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        answer,
        json!({
            "versions": 1,
            "movies": 1,
            "series": 0,
            "seasons": 0,
            "episodes": 0,
            "wiped_users": 0
        })
    );
    assert!(stores.catalog.get_movie(&MovieId("m-gone".into())).await.unwrap().is_none());
    assert!(stores.catalog.get_movie(&MovieId("m-old".into())).await.unwrap().is_some());
    assert!(stores.user_data.forgotten().is_empty());
    assert!(logs_contain("User [admin] cleaned library [lib], user data wiped: [false]"));
}

#[traced_test]
#[tokio::test]
async fn cleaning_every_library_takes_what_a_deleted_library_left_and_can_wipe() {
    let stores = stores();
    with_missing_files(&stores);

    let wipe = Some(json!({ "wipe_user_data": true }));
    let (status, answer) = post(&stores, CLEAN_ALL, Some(ADMIN), wipe).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!((&answer["versions"], &answer["movies"]), (&json!(2), &json!(2)));
    assert_eq!(answer["wiped_users"], 1);
    assert!(stores.catalog.get_movie(&MovieId("m-old".into())).await.unwrap().is_none());
    assert!(stores.catalog.get_movie(&MovieId("m1".into())).await.unwrap().is_some());
    assert!(logs_contain("User [admin] cleaned every library, user data wiped: [true]"));
}

#[tokio::test]
async fn a_clean_can_wipe_one_librarys_user_data_and_waits_for_its_jobs() {
    let stores = stores();
    with_missing_files(&stores);

    let wipe = Some(json!({ "wipe_user_data": true }));
    let (status, answer) = post(&stores, CLEAN, Some(ADMIN), wipe).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(answer["wiped_users"], 1);

    seed(&stores, "scan-1", JobKind::LibraryScan, "lib".into(), JobStatus::Running);
    for uri in [CLEAN, CLEAN_ALL] {
        let (status, answer) = post(&stores, uri, Some(ADMIN), None).await;
        assert_eq!(status, StatusCode::CONFLICT, "{uri}");
        assert_eq!(answer["error"]["code"], "library_busy", "{uri}");
        assert_eq!(answer["jobs"][0]["id"], "scan-1", "{uri}");
    }
    assert!(stores.catalog.get_movie(&MovieId("m-old".into())).await.unwrap().is_some());
}

#[tokio::test]
async fn only_an_admin_purges_or_cancels_and_only_a_library_that_exists() {
    let stores = stores();
    for uri in [PURGE, CLEAN, CLEAN_ALL, CANCEL] {
        assert_eq!(post(&stores, uri, Some(USER), None).await.0, StatusCode::FORBIDDEN, "{uri}");
        assert_eq!(post(&stores, uri, None, None).await.0, StatusCode::UNAUTHORIZED, "{uri}");
    }
    for uri in [
        "/api/v1/libraries/missing/purge",
        "/api/v1/libraries/missing/clean",
        "/api/v1/libraries/missing/jobs/cancel",
    ] {
        let (status, answer) = post(&stores, uri, Some(ADMIN), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        assert_eq!(answer["error"]["code"], "not_found");
    }
    assert!(stores.catalog.get_movie(&MovieId("m1".into())).await.unwrap().is_some());
}

#[traced_test]
#[tokio::test]
async fn cancelling_answers_what_was_stopped() {
    let stores = stores();
    let search = SubtitleJobPayload {
        version_id: VersionId("v1".into()),
        imdb_id: None,
        title: None,
        languages: vec!["en".into()],
        season: None,
        episode: None,
        transcribe_on_miss: false,
    };
    let trickplay = TrickplayJobPayload {
        version_id: VersionId("v1".into()),
        source_path: "/m/alpha.mkv".into(),
        duration_ms: 1,
    };
    seed(&stores, "search", JobKind::Subtitles, search.encode(), JobStatus::Queued);
    seed(&stores, "sheets", JobKind::Trickplay, trickplay.encode(), JobStatus::Running);

    let (status, answer) = post(&stores, CANCEL, Some(ADMIN), None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(answer, json!({ "cancelled": 1, "signalled": 1, "running": 0 }));
    let search = stores.jobs.get(&JobId("search".into())).await.unwrap().unwrap();
    assert_eq!(search.status, JobStatus::Cancelled);
    assert!(logs_contain("User [admin] cancelled the jobs of library [lib], scope: [All]"));
}

#[tokio::test]
async fn cancelling_for_a_purge_or_a_clean_stops_only_what_blocks_it() {
    for (blocking, cancelled) in [("purge", 1), ("clean", 0)] {
        let stores = stores();
        let search = SubtitleJobPayload {
            version_id: VersionId("v1".into()),
            imdb_id: None,
            title: None,
            languages: vec!["en".into()],
            season: None,
            episode: None,
            transcribe_on_miss: false,
        };
        let trickplay = TrickplayJobPayload {
            version_id: VersionId("v1".into()),
            source_path: "/m/alpha.mkv".into(),
            duration_ms: 1,
        };
        seed(&stores, "search", JobKind::Subtitles, search.encode(), JobStatus::Queued);
        seed(&stores, "sheets", JobKind::Trickplay, trickplay.encode(), JobStatus::Queued);

        let body = Some(json!({ "blocking": blocking }));
        let (status, answer) = post(&stores, CANCEL, Some(ADMIN), body).await;

        assert_eq!(status, StatusCode::OK, "{blocking}");
        assert_eq!(answer, json!({ "cancelled": cancelled, "signalled": 0, "running": 0 }));
        let sheets = stores.jobs.get(&JobId("sheets".into())).await.unwrap().unwrap();
        assert_eq!(sheets.status, JobStatus::Queued, "{blocking}");
    }
}

#[tokio::test]
async fn a_library_held_by_a_status_answers_409_with_what_holds_it() {
    for (held, code) in [
        (ScanStatus::Running, "scan_in_progress"),
        (ScanStatus::Purging, "library_purging"),
        (ScanStatus::Cleaning, "library_cleaning"),
    ] {
        let stores = stores();
        let state = ScanState {
            library: LibraryId("lib".into()),
            status: held,
            progress: 0.0,
            started_at: None,
            last_scanned_at: None,
            error: None,
        };
        stores.libraries.save_scan_state(state).await.unwrap();

        for uri in [PURGE, CLEAN, CLEAN_ALL] {
            let (status, answer) = post(&stores, uri, Some(ADMIN), None).await;
            assert_eq!(status, StatusCode::CONFLICT, "{held:?} {uri}");
            assert_eq!(answer["error"]["code"], code, "{held:?} {uri}");
            assert!(answer.get("jobs").is_none(), "{held:?} {uri}");
        }
    }
}

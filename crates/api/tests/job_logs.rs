use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use tower::ServiceExt;

use api::{AppState, JobLogState, job_log_router};
use contracts::Generator;
use domain::job::{JobId, JobLogLevel, JobLogStore};
use jiff::Timestamp;
use mocks::MockJobLogStore;

const JOB: &str = "job-scan";
const ADMIN: &str = "Bearer access:admin";
const USER: &str = "Bearer access:u1";

async fn seeded_store() -> MockJobLogStore {
    let store = MockJobLogStore::new();
    let job = JobId(JOB.into());
    let at = Timestamp::UNIX_EPOCH;
    store
        .append(&job, at, JobLogLevel::Info, "started LibraryScan")
        .await
        .unwrap();
    store
        .append(&job, at, JobLogLevel::Info, "succeeded")
        .await
        .unwrap();
    store
}

async fn seeded_multi_level_store() -> MockJobLogStore {
    let store = MockJobLogStore::new();
    let job = JobId(JOB.into());
    let at = Timestamp::UNIX_EPOCH;
    for (level, message) in [
        (JobLogLevel::Debug, "probing"),
        (JobLogLevel::Info, "started"),
        (JobLogLevel::Warn, "slow"),
        (JobLogLevel::Error, "boom"),
    ] {
        store.append(&job, at, level, message).await.unwrap();
    }
    store
}

fn app(store: MockJobLogStore) -> Router {
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
    job_log_router(state, JobLogState::new(store))
}

async fn send(app: Router, method: &str, uri: &str, auth: Option<&str>) -> (StatusCode, Vec<u8>) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = auth {
        builder = builder.header(header::AUTHORIZATION, token);
    }
    let response = app
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    (status, body)
}

fn lines(body: &[u8]) -> Vec<String> {
    let value: Value = serde_json::from_slice(body).unwrap();
    value["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l.as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn admin_reads_seeded_lines() {
    let store = seeded_store().await;
    let (status, body) = send(
        app(store),
        "GET",
        "/api/v1/admin/jobs/job-scan/logs",
        Some(ADMIN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let lines = lines(&body);
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("started LibraryScan"));
    assert!(lines[1].contains("succeeded"));
}

#[tokio::test]
async fn tail_limits_returned_lines() {
    let store = seeded_store().await;
    let (status, body) = send(
        app(store),
        "GET",
        "/api/v1/admin/jobs/job-scan/logs?tail=1",
        Some(ADMIN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let lines = lines(&body);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("succeeded"));
}

#[tokio::test]
async fn wipe_clears_then_read_is_empty() {
    let store = seeded_store().await;
    let (status, _) = send(
        app(store.clone()),
        "DELETE",
        "/api/v1/admin/jobs/job-scan/logs",
        Some(ADMIN),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, body) = send(
        app(store),
        "GET",
        "/api/v1/admin/jobs/job-scan/logs",
        Some(ADMIN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(lines(&body).is_empty());
}

#[tokio::test]
async fn level_filter_returns_that_severity_and_above() {
    let store = seeded_multi_level_store().await;
    let (status, body) = send(
        app(store),
        "GET",
        "/api/v1/admin/jobs/job-scan/logs?level=warn",
        Some(ADMIN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let lines = lines(&body);
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("slow"));
    assert!(lines[1].contains("boom"));
}

#[tokio::test]
async fn level_debug_returns_everything() {
    let store = seeded_multi_level_store().await;
    let (status, body) = send(
        app(store),
        "GET",
        "/api/v1/admin/jobs/job-scan/logs?level=debug",
        Some(ADMIN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(lines(&body).len(), 4);
}

#[tokio::test]
async fn unknown_level_is_bad_request() {
    let store = seeded_multi_level_store().await;
    let (status, _) = send(
        app(store),
        "GET",
        "/api/v1/admin/jobs/job-scan/logs?level=verbose",
        Some(ADMIN),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn non_admin_is_forbidden() {
    let store = seeded_store().await;
    let (status, _) = send(
        app(store),
        "GET",
        "/api/v1/admin/jobs/job-scan/logs",
        Some(USER),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn missing_auth_is_unauthorized() {
    let store = seeded_store().await;
    let (status, _) = send(app(store), "GET", "/api/v1/admin/jobs/job-scan/logs", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

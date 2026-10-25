use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use jiff::civil::date;
use serde_json::{Value, json};
use tower::ServiceExt;
use tracing_test::traced_test;

use api::{AppState, ServerLogState, bounded, server_log_router};
use contracts::Generator;
use domain::common::LogLevel;
use domain::diagnostics::{ServerLogCursor, ServerLogFrom, ServerLogPage, ServerLogRequest};
use mocks::MockServerLogStore;

const ADMIN: &str = "Bearer access:admin";
const USER: &str = "Bearer access:u1";
const LOGS: &str = "/api/v1/admin/server/logs";

fn cursor(hour: i8, entry: u64) -> ServerLogCursor {
    ServerLogCursor { hour: date(2026, 10, 24).at(hour, 0, 0, 0), entry }
}

fn page() -> ServerLogPage {
    ServerLogPage {
        entries: vec!["2026-10-24T09:00:00Z  INFO server: started".into()],
        before: Some(cursor(8, 3)),
        after: Some(cursor(9, 1)),
        more: false,
    }
}

fn app(store: MockServerLogStore) -> Router {
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
    bounded(server_log_router(state, ServerLogState::new(store)), Duration::from_secs(60))
}

async fn get(store: &MockServerLogStore, uri: &str, auth: Option<&str>) -> (StatusCode, Value) {
    let mut builder = Request::builder().method("GET").uri(uri);
    if let Some(token) = auth {
        builder = builder.header(header::AUTHORIZATION, token);
    }
    let response = app(store.clone()).oneshot(builder.body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

#[traced_test]
#[tokio::test]
async fn an_admin_reads_the_newest_entries_with_their_cursors() {
    let store = MockServerLogStore::with_page(page());

    let (status, body) = get(&store, LOGS, Some(ADMIN)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({
            "entries": ["2026-10-24T09:00:00Z  INFO server: started"],
            "before": "2026-10-24-08:3",
            "after": "2026-10-24-09:1",
            "more": false
        })
    );
    let request = ServerLogRequest { from: ServerLogFrom::Newest, limit: 200, min_level: None };
    assert_eq!(store.requests(), vec![request]);
    assert!(!logs_contain("User [admin]"), "reading the log adds nothing to it");
}

#[tokio::test]
async fn only_an_admin_reads_them() {
    let store = MockServerLogStore::with_page(page());

    assert_eq!(get(&store, LOGS, None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(get(&store, LOGS, Some(USER)).await.0, StatusCode::FORBIDDEN);
    assert!(store.requests().is_empty());
}

#[tokio::test]
async fn the_cursors_the_limit_and_the_level_reach_the_store() {
    let store = MockServerLogStore::new();

    for uri in [
        format!("{LOGS}?before=2026-10-24-08:3&limit=50&level=warn"),
        format!("{LOGS}?after=2026-10-24-09:1&level=ERROR"),
        format!("{LOGS}?limit=0"),
        format!("{LOGS}?limit=5000"),
    ] {
        assert_eq!(get(&store, &uri, Some(ADMIN)).await.0, StatusCode::OK, "{uri}");
    }

    let asked = store.requests();
    assert_eq!(asked[0].from, ServerLogFrom::Before(cursor(8, 3)));
    assert_eq!((asked[0].limit, asked[0].min_level), (50, Some(LogLevel::Warn)));
    assert_eq!(asked[1].from, ServerLogFrom::After(cursor(9, 1)));
    assert_eq!((asked[1].limit, asked[1].min_level), (200, Some(LogLevel::Error)));
    assert_eq!(asked[2].limit, 1);
    assert_eq!(asked[3].limit, 1000);
}

#[tokio::test]
async fn a_bad_request_never_reaches_the_store() {
    let store = MockServerLogStore::new();

    for uri in [
        format!("{LOGS}?before=2026-10-24-08:3&after=2026-10-24-09:1"),
        format!("{LOGS}?level=verbose"),
        format!("{LOGS}?before=../../etc/passwd:1"),
        format!("{LOGS}?after=2026-10-24-09:-1"),
    ] {
        let (status, body) = get(&store, &uri, Some(ADMIN)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}");
        assert_eq!(body["error"]["code"], "bad_request", "{uri}");
    }
    assert!(store.requests().is_empty());
}

#[traced_test]
#[tokio::test]
async fn a_cursor_the_store_refuses_is_a_bad_request() {
    let store = MockServerLogStore::refusing_cursors();

    let (status, _) = get(&store, &format!("{LOGS}?after=2026-10-24-09:99"), Some(ADMIN)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(!logs_contain("User [admin]"), "a refused read adds nothing to the log either");
}

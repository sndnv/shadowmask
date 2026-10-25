use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use jiff::{SignedDuration, Timestamp};
use serde_json::{Value, json};
use tower::ServiceExt;
use tracing_test::traced_test;

use api::{AppState, ReportState, bounded, report_router};
use contracts::Generator;
use domain::diagnostics::{PlayerId, Report, ReportId, ReportSummary};
use domain::user::{Device, DeviceId, UserId};
use mocks::MockReportRepo;

const ADMIN: &str = "Bearer access:admin";
const USER: &str = "Bearer access:u1";
const ROKU: &str = "Bearer device:roku-1";
const SUBMIT: &str = "/api/v1/diagnostics/reports";
const LIST: &str = "/api/v1/admin/diagnostics/reports";

fn hours_ago(hours: i64) -> Timestamp {
    Timestamp::now().checked_sub(SignedDuration::from_hours(hours)).unwrap()
}

fn stored(id: &str, created_at: Timestamp) -> Report {
    Report {
        summary: ReportSummary {
            id: ReportId(id.into()),
            created_at,
            user: UserId("u1".into()),
            player: PlayerId("player-1".into()),
            app: "flutter".into(),
            platform: "Android 14".into(),
            app_version: "0.0.7".into(),
            note: None,
        },
        payload: "{\"schema\":1}".into(),
    }
}

fn body(player_id: Option<&str>, note: Option<&str>, payload: Value) -> String {
    json!({
        "player_id": player_id,
        "app": "flutter",
        "platform": "Android 14",
        "app_version": "0.0.7",
        "note": note,
        "payload": payload
    })
    .to_string()
}

fn app(reports: &MockReportRepo) -> Router {
    let generator = Generator::new();
    generator.auth.add_device(Device {
        id: DeviceId("roku-1".into()),
        user: UserId("u1".into()),
        name: "Living room".into(),
        platform: "roku".into(),
        created_at: Timestamp::UNIX_EPOCH,
        last_seen: None,
        target: None,
    });
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
    let reports = ReportState::new(reports.clone(), SignedDuration::from_hours(24));
    bounded(report_router(state, reports), Duration::from_secs(60))
}

async fn send(
    reports: &MockReportRepo,
    method: &str,
    uri: &str,
    auth: Option<&str>,
    body: Option<String>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(auth) = auth {
        builder = builder.header(header::AUTHORIZATION, auth);
    }
    let body = match body {
        Some(body) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(body)
        }
        None => Body::empty(),
    };
    let response = app(reports).oneshot(builder.body(body).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[traced_test]
#[tokio::test]
async fn a_signed_in_user_sends_a_report_under_their_player_id() {
    let reports = MockReportRepo::new();
    let payload = json!({ "schema": 1, "log": ["09:00:01 loaded"] });

    let sent = body(Some(" player-1 "), Some("  stutters after a seek "), payload.clone());
    let (status, answer) = send(&reports, "POST", SUBMIT, Some(USER), Some(sent)).await;

    assert_eq!(status, StatusCode::CREATED);
    let report = reports.stored().pop().unwrap();
    assert_eq!(answer, json!({ "id": report.summary.id.0 }));
    assert_eq!(report.summary.user, UserId("u1".into()));
    assert_eq!(report.summary.player, PlayerId("player-1".into()));
    assert_eq!(
        (report.summary.app.as_str(), report.summary.platform.as_str()),
        ("flutter", "Android 14")
    );
    assert_eq!(report.summary.app_version, "0.0.7");
    assert_eq!(report.summary.note.as_deref(), Some("stutters after a seek"));
    assert_eq!(serde_json::from_str::<Value>(&report.payload).unwrap(), payload);
    assert!(logs_contain(&format!(
        "User [u1] sent diagnostics report [{}] from player [player-1]",
        report.summary.id.0
    )));
}

#[tokio::test]
async fn a_blank_note_is_no_note() {
    let reports = MockReportRepo::new();

    let sent = body(Some("player-1"), Some("   "), json!({}));
    assert_eq!(send(&reports, "POST", SUBMIT, Some(USER), Some(sent)).await.0, StatusCode::CREATED);

    assert_eq!(reports.stored()[0].summary.note, None);
}

#[tokio::test]
async fn a_linked_device_is_its_own_player_whatever_the_body_says() {
    let reports = MockReportRepo::new();

    let sent = body(Some("something-else"), None, json!({}));
    assert_eq!(send(&reports, "POST", SUBMIT, Some(ROKU), Some(sent)).await.0, StatusCode::CREATED);
    let sent = body(None, None, json!({}));
    assert_eq!(send(&reports, "POST", SUBMIT, Some(ROKU), Some(sent)).await.0, StatusCode::CREATED);

    let players: Vec<PlayerId> = reports.stored().into_iter().map(|r| r.summary.player).collect();
    assert_eq!(players, vec![PlayerId("roku-1".into()), PlayerId("roku-1".into())]);
}

#[traced_test]
#[tokio::test]
async fn a_password_sign_in_without_a_player_id_is_refused() {
    let reports = MockReportRepo::new();

    for player in [None, Some(""), Some("   ")] {
        let sent = body(player, None, json!({}));
        let (status, answer) = send(&reports, "POST", SUBMIT, Some(USER), Some(sent)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{player:?}");
        assert_eq!(answer["error"]["message"], "a player id is required");
    }

    assert!(reports.stored().is_empty());
    assert!(logs_contain("User [u1] failed to send a diagnostics report: a player id is required"));
}

#[traced_test]
#[tokio::test]
async fn a_payload_that_is_not_an_object_is_refused() {
    let reports = MockReportRepo::new();

    for payload in [json!([]), json!("text"), json!(1), Value::Null] {
        let sent = body(Some("player-1"), None, payload.clone());
        let (status, _) = send(&reports, "POST", SUBMIT, Some(USER), Some(sent)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{payload}");
    }

    assert!(reports.stored().is_empty());
    assert!(logs_contain(
        "User [u1] failed to send a diagnostics report: the payload is not a JSON object"
    ));
}

#[tokio::test]
async fn sending_needs_a_sign_in() {
    let reports = MockReportRepo::new();

    let sent = body(Some("player-1"), None, json!({}));
    assert_eq!(send(&reports, "POST", SUBMIT, None, Some(sent)).await.0, StatusCode::UNAUTHORIZED);
    assert!(reports.stored().is_empty());
}

#[traced_test]
#[tokio::test]
async fn a_store_that_fails_answers_500_on_every_route() {
    let reports = MockReportRepo::new();
    reports.set_fail();
    let one = format!("{LIST}/r1");

    let sent = body(Some("player-1"), None, json!({}));
    let calls = [
        ("POST", SUBMIT, USER, Some(sent)),
        ("GET", LIST, ADMIN, None),
        ("GET", one.as_str(), ADMIN, None),
        ("DELETE", one.as_str(), ADMIN, None),
    ];
    for (method, uri, auth, sent) in calls {
        let (status, _) = send(&reports, method, uri, Some(auth), sent).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{method} {uri}");
    }

    for action in [
        "User [u1] failed to send a diagnostics report: ",
        "User [admin] failed to retrieve diagnostics reports: ",
        "User [admin] failed to retrieve diagnostics report: ",
        "User [admin] failed to delete diagnostics report: ",
    ] {
        assert!(logs_contain(action), "{action}");
    }
}

#[traced_test]
#[tokio::test]
async fn an_admin_lists_the_kept_reports_newest_first() {
    let reports = MockReportRepo::new();
    reports.seed(stored("older", hours_ago(2)));
    reports.seed(stored("newer", hours_ago(1)));
    reports.seed(stored("expired", hours_ago(25)));

    let (status, answer) = send(&reports, "GET", LIST, Some(ADMIN), None).await;

    assert_eq!(status, StatusCode::OK);
    let ids: Vec<&str> =
        answer["items"].as_array().unwrap().iter().map(|r| r["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["newer", "older"], "a report past the 24 h window is not shown");
    assert_eq!(answer["total"], 2);
    let first = &answer["items"][0];
    assert_eq!(first["user_id"], "u1");
    assert_eq!(first["player_id"], "player-1");
    assert_eq!(first["app_version"], "0.0.7");
    assert!(first.get("payload").is_none(), "the list leaves the payloads out");
    assert!(logs_contain("User [admin] retrieved 2 of 2 diagnostics reports"));

    let (_, second) =
        send(&reports, "GET", &format!("{LIST}?offset=1&limit=1"), Some(ADMIN), None).await;
    assert_eq!(second["items"][0]["id"], "older");
    assert_eq!((second["offset"].as_u64(), second["limit"].as_u64()), (Some(1), Some(1)));
}

#[tokio::test]
async fn the_list_filters_before_paging_and_a_blank_filter_is_none() {
    let reports = MockReportRepo::new();
    reports.seed(stored("flutter-1", hours_ago(1)));
    let mut roku = stored("roku-1", hours_ago(2));
    roku.summary.app = "roku".into();
    roku.summary.platform = "Roku Ultra (4800X)".into();
    roku.summary.note = Some("android was fine".into());
    reports.seed(roku);

    let (status, answer) =
        send(&reports, "GET", &format!("{LIST}?filter=ROKU"), Some(ADMIN), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(answer["total"], 1);
    assert_eq!(answer["items"][0]["id"], "roku-1");

    let (_, android) =
        send(&reports, "GET", &format!("{LIST}?filter=android"), Some(ADMIN), None).await;
    assert_eq!(answer_ids(&android), ["flutter-1"], "the note is not searched");

    let (_, blank) =
        send(&reports, "GET", &format!("{LIST}?filter=%20%20"), Some(ADMIN), None).await;
    assert_eq!(blank["total"], 2);
}

fn answer_ids(answer: &Value) -> Vec<&str> {
    answer["items"].as_array().unwrap().iter().map(|r| r["id"].as_str().unwrap()).collect()
}

#[traced_test]
#[tokio::test]
async fn an_admin_reads_one_report_with_its_payload() {
    let reports = MockReportRepo::new();
    reports.seed(stored("r1", hours_ago(1)));
    reports.seed(stored("expired", hours_ago(25)));

    let (status, answer) = send(&reports, "GET", &format!("{LIST}/r1"), Some(ADMIN), None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(answer["id"], "r1");
    assert_eq!(answer["platform"], "Android 14");
    assert_eq!(answer["payload"], json!({ "schema": 1 }));
    assert!(logs_contain("User [admin] retrieved diagnostics report [r1]"));

    for gone in ["missing", "expired"] {
        let (status, _) = send(&reports, "GET", &format!("{LIST}/{gone}"), Some(ADMIN), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{gone}");
    }
}

#[traced_test]
#[tokio::test]
async fn an_admin_deletes_a_report_once() {
    let reports = MockReportRepo::new();
    reports.seed(stored("r1", hours_ago(1)));
    let one = format!("{LIST}/r1");

    assert_eq!(send(&reports, "DELETE", &one, Some(ADMIN), None).await.0, StatusCode::NO_CONTENT);
    assert!(reports.stored().is_empty());
    assert!(logs_contain("User [admin] deleted diagnostics report [r1]"));

    assert_eq!(send(&reports, "DELETE", &one, Some(ADMIN), None).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn only_an_admin_reads_or_deletes_reports() {
    let reports = MockReportRepo::new();
    reports.seed(stored("r1", hours_ago(1)));
    let one = format!("{LIST}/r1");

    for (method, uri) in [("GET", LIST), ("GET", one.as_str()), ("DELETE", one.as_str())] {
        assert_eq!(send(&reports, method, uri, None, None).await.0, StatusCode::UNAUTHORIZED);
        for auth in [USER, ROKU] {
            let (status, _) = send(&reports, method, uri, Some(auth), None).await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{method} {uri} {auth}");
        }
    }
    assert_eq!(reports.stored().len(), 1);
}

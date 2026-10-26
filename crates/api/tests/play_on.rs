use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use api::{AppState, PlayOnState, bounded, play_on_router, router};
use contracts::Generator;
use domain::catalog::{MovieId, TitleId, Version, VersionId};
use domain::common::Quality;
use domain::library::LibraryId;
use domain::session::{PlayCommandClaims, PlayCommandTokens};
use domain::user::{Device, DeviceId, DeviceTarget, UserId};
use jiff::Timestamp;
use mocks::{MockPlayCommandTokens, MockUserRepo};

const USER: &str = "Bearer access:u1";
const ADMIN: &str = "Bearer access:admin";
const ROKU: &str = "Bearer device:roku-1";
const PHONE: &str = "Bearer device:phone-1";
const OTHER: &str = "Bearer device:other-1";

struct Harness {
    app: Router,
    tokens: MockPlayCommandTokens,
    users: MockUserRepo,
}

fn version(id: &str, library: &str) -> Version {
    Version {
        id: VersionId(id.into()),
        title: TitleId::Movie(MovieId("m1".into())),
        library: LibraryId(library.into()),
        quality: Quality::Fhd,
        container: "mkv".into(),
        path: format!("/media/{id}.mkv"),
        size_bytes: 1,
        duration_ms: 1000,
        available: true,
        added_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
    }
}

fn roku_target() -> DeviceTarget {
    DeviceTarget::new(
        "roku",
        &["192.168.1.40".to_owned()],
        8060,
        Some("dev".into()),
        Timestamp::UNIX_EPOCH,
    )
    .unwrap()
}

fn device(id: &str, user: &str, target: Option<DeviceTarget>) -> Device {
    Device {
        id: DeviceId(id.into()),
        user: UserId(user.into()),
        name: id.into(),
        platform: "roku".into(),
        created_at: Timestamp::UNIX_EPOCH,
        last_seen: None,
        target,
    }
}

fn harness(roku_reported: bool) -> Harness {
    let generator = Generator::new();
    generator.catalog_repo.add_version(version("hidden", "lib2"));
    generator.catalog_repo.add_version(Version { available: false, ..version("gone", "lib1") });
    generator.auth.add_device(device("roku-1", "u1", roku_reported.then(roku_target)));
    generator.auth.add_device(device("phone-1", "u1", None));
    generator.auth.add_device(device("other-1", "admin", Some(roku_target())));
    generator.auth.add_device(device("den-1", "admin", None));
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
    let tokens = MockPlayCommandTokens::new();
    let app = router(state.clone())
        .merge(play_on_router(state.clone(), PlayOnState::new(state, tokens.clone())));
    Harness { app: bounded(app, Duration::from_secs(60)), tokens, users: generator.users_repo }
}

async fn send(
    app: &Router,
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
    let response = app.clone().oneshot(builder.body(body).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

fn command(device: &str, user: &str, expires_in: i64) -> PlayCommandClaims {
    PlayCommandClaims {
        user: UserId(user.into()),
        device: DeviceId(device.into()),
        version: VersionId("v1".into()),
        expires_at: Timestamp::from_second(Timestamp::now().as_second() + expires_in).unwrap(),
        nonce: "n1".into(),
    }
}

async fn report(app: &Router, auth: &str, body: Value) -> (StatusCode, Value) {
    send(app, "PUT", "/api/v1/devices/self/target", Some(auth), Some(body.to_string())).await
}

async fn mint(app: &Router, device: &str, version: &str) -> (StatusCode, Value) {
    let body = json!({ "version_id": version }).to_string();
    send(app, "POST", &format!("/api/v1/devices/{device}/commands"), Some(USER), Some(body)).await
}

async fn redeem(app: &Router, auth: &str, token: &str) -> (StatusCode, Value) {
    let body = json!({ "token": token }).to_string();
    send(app, "POST", "/api/v1/devices/self/commands/redeem", Some(auth), Some(body)).await
}

async fn share(app: &Router, auth: &str, device: &str, username: &str) -> (StatusCode, Value) {
    let body = json!({ "username": username }).to_string();
    send(app, "PUT", &format!("/api/v1/devices/{device}/shares"), Some(auth), Some(body)).await
}

async fn unshare(app: &Router, auth: &str, device: &str, user: &str) -> (StatusCode, Value) {
    send(app, "DELETE", &format!("/api/v1/devices/{device}/shares/{user}"), Some(auth), None).await
}

async fn shared_with(app: &Router, device: &str) -> Value {
    let (status, devices) = send(app, "GET", "/api/v1/users/u1/devices", Some(USER), None).await;
    assert_eq!(status, StatusCode::OK);
    let found = devices.as_array().unwrap().iter().find(|d| d["id"] == device).unwrap();
    found["shared_with"].clone()
}

async fn shared_targets(app: &Router, auth: &str) -> Value {
    let (status, targets) = send(app, "GET", "/api/v1/devices/shared", Some(auth), None).await;
    assert_eq!(status, StatusCode::OK);
    targets
}

#[tokio::test]
async fn a_device_reports_itself_and_the_device_list_carries_the_report() {
    let harness = harness(false);
    let report_body = json!({
        "kind": "roku", "addresses": ["192.168.1.40", "10.0.0.2"], "port": 8060, "app_id": "dev"
    });
    let (status, _) = report(&harness.app, ROKU, report_body).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, devices) =
        send(&harness.app, "GET", "/api/v1/users/u1/devices", Some(USER), None).await;
    assert_eq!(status, StatusCode::OK);
    let roku = devices.as_array().unwrap().iter().find(|d| d["id"] == "roku-1").unwrap();
    assert_eq!(roku["target"]["kind"], "roku");
    assert_eq!(roku["target"]["addresses"], json!(["192.168.1.40", "10.0.0.2"]));
    assert_eq!(roku["target"]["port"], 8060);
    assert_eq!(roku["target"]["app_id"], "dev");
    assert!(roku["target"]["reported_at"].is_string());
    let phone = devices.as_array().unwrap().iter().find(|d| d["id"] == "phone-1").unwrap();
    assert!(phone["target"].is_null());
}

#[tokio::test]
async fn a_password_login_cannot_report_a_target() {
    let harness = harness(false);
    let body = json!({ "kind": "android", "addresses": ["10.0.0.2"], "port": 7979 });
    let (status, error) = report(&harness.app, USER, body).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(error["error"]["code"], "forbidden");
}

#[tokio::test]
async fn a_report_breaking_a_rule_is_a_bad_request_and_a_wrong_shape_is_unprocessable() {
    let harness = harness(false);
    for broken in [
        json!({ "kind": "roku", "addresses": ["8.8.8.8"], "port": 8060, "app_id": "dev" }),
        json!({ "kind": "roku", "addresses": ["10.0.0.2"], "port": 8060 }),
        json!({ "kind": "android", "addresses": [], "port": 7979 }),
        json!({ "kind": "android", "addresses": ["10.0.0.2"], "port": 0 }),
        json!({ "kind": "cast", "addresses": ["10.0.0.2"], "port": 8009 }),
    ] {
        let (status, error) = report(&harness.app, ROKU, broken.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{broken}");
        assert_eq!(error["error"]["code"], "bad_request", "{broken}");
    }
    let (status, _) =
        report(&harness.app, ROKU, json!({ "kind": "roku", "addresses": "10.0.0.2" })).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn clearing_a_target_answers_no_content_with_or_without_one() {
    let harness = harness(true);
    for _ in 0..2 {
        let (status, _) =
            send(&harness.app, "DELETE", "/api/v1/devices/self/target", Some(ROKU), None).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }
    let (status, error) = mint(&harness.app, "roku-1", "v1").await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["error"]["code"], "not_a_target");
}

#[tokio::test]
async fn a_command_names_its_target_and_lasts_a_minute() {
    let harness = harness(true);
    let before = Timestamp::now().as_second();
    let (status, body) = mint(&harness.app, "roku-1", "v1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["target"],
        json!({ "kind": "roku", "addresses": ["192.168.1.40"], "port": 8060, "app_id": "dev" })
    );

    let claims = harness.tokens.verify(body["token"].as_str().unwrap()).unwrap();
    assert_eq!(claims.user, UserId("u1".into()));
    assert_eq!(claims.device, DeviceId("roku-1".into()));
    assert_eq!(claims.version, VersionId("v1".into()));
    let lifetime = claims.expires_at.as_second() - before;
    assert!((59..=61).contains(&lifetime), "{lifetime}");
    let expires_at: Timestamp = body["expires_at"].as_str().unwrap().parse().unwrap();
    assert_eq!(expires_at, claims.expires_at);
}

#[tokio::test]
async fn a_command_for_another_users_or_an_unknown_device_is_not_found() {
    let harness = harness(true);
    for device in ["other-1", "ghost"] {
        let (status, error) = mint(&harness.app, device, "v1").await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{device}");
        assert_eq!(error["error"]["code"], "not_found", "{device}");
    }
}

#[tokio::test]
async fn a_device_that_never_reported_is_not_a_target() {
    let harness = harness(true);
    let (status, error) = mint(&harness.app, "phone-1", "v1").await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["error"]["code"], "not_a_target");
}

#[tokio::test]
async fn a_version_the_caller_cannot_play_is_version_not_found() {
    let harness = harness(true);
    for version in ["hidden", "nope", "gone"] {
        let (status, error) = mint(&harness.app, "roku-1", version).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{version}");
        assert_eq!(error["error"]["code"], "version_not_found", "{version}");
    }
}

#[tokio::test]
async fn minting_needs_a_login_and_a_working_signer() {
    let harness = harness(true);
    let body = json!({ "version_id": "v1" }).to_string();
    let (status, _) =
        send(&harness.app, "POST", "/api/v1/devices/roku-1/commands", None, Some(body)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    harness.tokens.set_fail();
    let (status, _) = mint(&harness.app, "roku-1", "v1").await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn the_named_device_redeems_its_command() {
    let harness = harness(true);
    let token = harness.tokens.create(&command("roku-1", "u1", 60)).unwrap();
    let (status, body) = redeem(&harness.app, ROKU, &token.0).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "version_id": "v1" }));
}

#[tokio::test]
async fn only_the_named_device_of_the_named_user_redeems() {
    let harness = harness(true);
    let token = harness.tokens.create(&command("roku-1", "u1", 60)).unwrap();
    let (status, error) = redeem(&harness.app, PHONE, &token.0).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(error["error"]["code"], "invalid_play_command");

    let (status, error) = redeem(&harness.app, USER, &token.0).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(error["error"]["code"], "forbidden");

    let foreign = harness.tokens.create(&command("roku-1", "admin", 60)).unwrap();
    let (status, error) = redeem(&harness.app, ROKU, &foreign.0).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(error["error"]["code"], "invalid_play_command");
}

#[tokio::test]
async fn an_expired_or_garbled_command_is_refused() {
    let harness = harness(true);
    let expired = harness.tokens.create(&command("roku-1", "u1", -60)).unwrap();
    let (status, error) = redeem(&harness.app, ROKU, &expired.0).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(error["error"]["code"], "play_command_expired");

    let (status, error) = redeem(&harness.app, ROKU, "garbage").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(error["error"]["code"], "invalid_play_command");
}

#[tokio::test]
async fn an_owner_shares_a_device_by_username_and_the_device_list_shows_it_until_unshared() {
    let harness = harness(true);
    for _ in 0..2 {
        let (status, body) = share(&harness.app, USER, "roku-1", "admin").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, json!({ "user_id": "admin", "username": "admin" }));
    }
    assert_eq!(
        shared_with(&harness.app, "roku-1").await,
        json!([{ "user_id": "admin", "username": "admin" }])
    );
    assert_eq!(shared_with(&harness.app, "phone-1").await, json!([]));

    let (status, _) = unshare(&harness.app, USER, "roku-1", "admin").await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(shared_with(&harness.app, "roku-1").await, json!([]));
}

#[tokio::test]
async fn sharing_refuses_an_unknown_user_oneself_and_a_device_the_caller_does_not_own() {
    let harness = harness(true);
    let (status, error) = share(&harness.app, USER, "roku-1", "nobody").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["error"]["code"], "user_not_found");

    let (status, error) = share(&harness.app, USER, "roku-1", "user").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "bad_request");

    for device in ["other-1", "ghost"] {
        let (status, error) = share(&harness.app, USER, device, "admin").await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{device}");
        assert_eq!(error["error"]["code"], "not_found", "{device}");
        let (status, error) = unshare(&harness.app, USER, device, "admin").await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{device}");
        assert_eq!(error["error"]["code"], "not_found", "{device}");
    }
}

#[tokio::test]
async fn an_admin_cannot_share_or_unshare_another_users_device() {
    let harness = harness(true);
    let (status, error) = share(&harness.app, ADMIN, "roku-1", "user").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["error"]["code"], "not_found");

    let (status, _) = share(&harness.app, USER, "roku-1", "admin").await;
    assert_eq!(status, StatusCode::OK);
    let (status, error) = unshare(&harness.app, ADMIN, "roku-1", "admin").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["error"]["code"], "not_found");
    assert_eq!(
        shared_with(&harness.app, "roku-1").await,
        json!([{ "user_id": "admin", "username": "admin" }])
    );
}

#[tokio::test]
async fn a_linked_device_cannot_share_or_unshare() {
    let harness = harness(true);
    let (status, error) = share(&harness.app, ROKU, "roku-1", "admin").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(error["error"]["code"], "forbidden");

    let (status, error) = unshare(&harness.app, ROKU, "roku-1", "admin").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(error["error"]["code"], "forbidden");
}

#[tokio::test]
async fn the_shared_list_holds_the_reported_devices_shared_with_the_caller() {
    let harness = harness(true);
    assert_eq!(shared_targets(&harness.app, USER).await, json!([]));
    for device in ["other-1", "den-1"] {
        let (status, _) = share(&harness.app, ADMIN, device, "user").await;
        assert_eq!(status, StatusCode::OK, "{device}");
    }

    assert_eq!(
        shared_targets(&harness.app, USER).await,
        json!([{
            "id": "other-1",
            "name": "other-1",
            "platform": "roku",
            "owner": "admin",
            "target": {
                "kind": "roku",
                "addresses": ["192.168.1.40"],
                "port": 8060,
                "app_id": "dev",
                "reported_at": "1970-01-01T00:00:00Z"
            }
        }])
    );
    assert_eq!(shared_targets(&harness.app, ADMIN).await, json!([]));
    let (status, _) = send(&harness.app, "GET", "/api/v1/devices/shared", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_command_to_a_shared_device_plays_as_its_owner_and_that_device_redeems_it() {
    let harness = harness(true);
    let (status, _) = share(&harness.app, ADMIN, "other-1", "user").await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = mint(&harness.app, "other-1", "v1").await;
    assert_eq!(status, StatusCode::OK);
    let claims = harness.tokens.verify(body["token"].as_str().unwrap()).unwrap();
    assert_eq!(claims.user, UserId("admin".into()));
    assert_eq!(claims.device, DeviceId("other-1".into()));

    let (status, redeemed) = redeem(&harness.app, OTHER, body["token"].as_str().unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(redeemed, json!({ "version_id": "v1" }));

    let (status, _) = unshare(&harness.app, ADMIN, "other-1", "u1").await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, error) = mint(&harness.app, "other-1", "v1").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["error"]["code"], "not_found");
}

#[tokio::test]
async fn a_version_the_devices_account_cannot_see_is_target_no_access() {
    let harness = harness(true);
    let libraries = [LibraryId("lib1".into()), LibraryId("lib2".into())];
    harness.users.grant(&UserId("u1".into()), &libraries);
    let (status, _) = share(&harness.app, ADMIN, "other-1", "user").await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = mint(&harness.app, "roku-1", "hidden").await;
    assert_eq!(status, StatusCode::OK);
    let (status, error) = mint(&harness.app, "other-1", "hidden").await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["error"]["code"], "target_no_access");

    let (status, error) = mint(&harness.app, "other-1", "gone").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["error"]["code"], "version_not_found");
}

#[tokio::test]
async fn a_version_only_the_devices_account_can_see_is_version_not_found() {
    let harness = harness(true);
    let libraries = [LibraryId("lib1".into()), LibraryId("lib2".into())];
    harness.users.grant(&UserId("admin".into()), &libraries);
    let (status, _) = share(&harness.app, ADMIN, "other-1", "user").await;
    assert_eq!(status, StatusCode::OK);

    let (status, error) = mint(&harness.app, "other-1", "hidden").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["error"]["code"], "version_not_found");
}

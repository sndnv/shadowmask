use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use api::{AppState, router};
use contracts::{Generator, requests};

fn build_app(generator: &Generator) -> Router {
    router(AppState::new(
        generator.auth.clone(),
        generator.catalog.clone(),
        generator.session.clone(),
        generator.library.clone(),
        generator.user.clone(),
        generator.user_library.clone(),
        generator.discovery.clone(),
    ))
}

fn method(name: &str) -> Method {
    match name {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "DELETE" => Method::DELETE,
        other => panic!("unsupported method {other}"),
    }
}

async fn call(
    app: Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request = match body {
        Some(body) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, value)
}

async fn mint_session(app: &Router) -> String {
    let (status, body) = call(
        app.clone(),
        Method::POST,
        "/api/v1/sessions",
        Some("access:u1"),
        Some(json!({
            "version_id": "v1",
            "capabilities": {"platform": "web", "profile_version": 1, "max_bitrate": null},
            "audio_track": 0,
            "subtitle": null
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    body["session_id"].as_str().unwrap().to_owned()
}

async fn mint_user(app: &Router) -> String {
    let (status, body) = call(
        app.clone(),
        Method::POST,
        "/api/v1/users",
        Some("access:admin"),
        Some(json!({"username": "minted", "password": "pw", "role": "user"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    body["id"].as_str().unwrap().to_owned()
}

fn snapshot(name: &str, status: StatusCode, body: Value) {
    let value = json!({"status": status.as_u16(), "body": body});
    insta::assert_json_snapshot!(name, value, {
        ".body.session_id" => "[session_id]",
        ".body.manifest_url" => "[manifest_url]",
        ".body.created_at" => "[created_at]",
        ".body.active[].session_id" => "[session_id]",
        ".body.active[].started_at" => "[started_at]",
        ".body.active[].last_heartbeat_at" => "[last_heartbeat_at]",
    });
}

#[tokio::test]
async fn endpoint_success_snapshots() {
    let shared = Generator::new();
    for endpoint in requests() {
        let generator = Generator::from_auth(shared.auth.clone());
        let app = build_app(&generator);
        let mut path = endpoint.path.to_owned();
        if path.contains("{session}") {
            let session = mint_session(&app).await;
            path = path.replace("{session}", &session);
        }
        if path.contains("{user}") {
            let user = mint_user(&app).await;
            path = path.replace("{user}", &user);
        }
        let (status, body) = call(
            app,
            method(endpoint.method),
            &path,
            endpoint.token.header(),
            endpoint.body.clone(),
        )
        .await;
        snapshot(endpoint.name, status, body);
    }
}

#[tokio::test]
async fn endpoint_error_snapshots() {
    let shared = Generator::new();
    let generator = Generator::from_auth(shared.auth.clone());
    let (status, body) = call(
        build_app(&generator),
        Method::GET,
        "/api/v1/movies",
        None,
        None,
    )
    .await;
    snapshot("err_missing_token", status, body);

    let (status, body) = call(
        build_app(&generator),
        Method::GET,
        "/api/v1/movies",
        Some("garbage"),
        None,
    )
    .await;
    snapshot("err_invalid_token", status, body);

    let (status, body) = call(
        build_app(&generator),
        Method::POST,
        "/api/v1/auth/login",
        None,
        Some(json!({"username": "admin", "password": "wrong"})),
    )
    .await;
    snapshot("err_invalid_credentials", status, body);

    let (status, body) = call(
        build_app(&generator),
        Method::POST,
        "/api/v1/auth/link",
        None,
        Some(json!({"code": "NOPE", "device": {"name": "Roku", "platform": "roku"}})),
    )
    .await;
    snapshot("err_unknown_link_code", status, body);

    let (status, body) = call(
        build_app(&generator),
        Method::GET,
        "/api/v1/users",
        Some("access:u1"),
        None,
    )
    .await;
    snapshot("err_forbidden", status, body);

    let (status, body) = call(
        build_app(&generator),
        Method::GET,
        "/api/v1/movies/ghost",
        Some("access:u1"),
        None,
    )
    .await;
    snapshot("err_not_found", status, body);

    let (status, body) = call(
        build_app(&generator),
        Method::POST,
        "/api/v1/sessions/does-not-exist/seek",
        Some("access:u1"),
        Some(json!({"position_ms": 1})),
    )
    .await;
    snapshot("err_session_not_found", status, body);

    let taken = build_app(&generator);
    let (created, _) = call(
        taken.clone(),
        Method::POST,
        "/api/v1/users",
        Some("access:admin"),
        Some(json!({"username": "dup", "password": "pw", "role": "user"})),
    )
    .await;
    assert_eq!(created, StatusCode::CREATED);
    let (status, body) = call(
        taken,
        Method::POST,
        "/api/v1/users",
        Some("access:admin"),
        Some(json!({"username": "dup", "password": "pw", "role": "user"})),
    )
    .await;
    snapshot("err_username_taken", status, body);

    let scanning = build_app(&generator);
    let (accepted, _) = call(
        scanning.clone(),
        Method::POST,
        "/api/v1/libraries/lib1/scan",
        Some("access:admin"),
        None,
    )
    .await;
    assert_eq!(accepted, StatusCode::ACCEPTED);
    let (status, body) = call(
        scanning,
        Method::POST,
        "/api/v1/libraries/lib1/scan",
        Some("access:admin"),
        None,
    )
    .await;
    snapshot("err_scan_in_progress", status, body);

    let limited = Generator::from_auth(shared.auth.clone());
    limited.session.set_concurrent_limit(1);
    let app = build_app(&limited);
    let start = json!({
        "version_id": "v1",
        "capabilities": {"platform": "web", "profile_version": 1, "max_bitrate": null},
        "audio_track": 0,
        "subtitle": null
    });
    let (first, _) = call(
        app.clone(),
        Method::POST,
        "/api/v1/sessions",
        Some("access:u1"),
        Some(start.clone()),
    )
    .await;
    assert_eq!(first, StatusCode::CREATED);
    let (status, body) = call(
        app,
        Method::POST,
        "/api/v1/sessions",
        Some("access:u1"),
        Some(start),
    )
    .await;
    snapshot("err_concurrent_limit", status, body);
}

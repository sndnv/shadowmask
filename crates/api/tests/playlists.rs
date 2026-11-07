use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use api::{AppState, PlaylistState, bounded, playlist_router, router};
use contracts::Generator;

fn app(generator: &Generator) -> Router {
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
    let playlists = PlaylistState::new(generator.playlist.clone());
    bounded(router(state.clone()).merge(playlist_router(state, playlists)), Duration::from_secs(60))
}

async fn call(app: &Router, method: Method, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, "Bearer access:u1");
    let request = match body {
        Some(body) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value =
        if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    (status, value)
}

fn cards(items: &Value) -> Vec<(String, String)> {
    items
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            let card = &item["card"];
            (card["type"].as_str().unwrap().to_owned(), card["id"].as_str().unwrap().to_owned())
        })
        .collect()
}

fn titled(kind: &str, id: &str) -> (String, String) {
    (kind.to_owned(), id.to_owned())
}

#[tokio::test]
async fn a_playlist_is_built_queued_and_leads_up_next() {
    let app = app(&Generator::new());

    let (status, created) =
        call(&app, Method::POST, "/api/v1/users/u1/playlists", Some(json!({"name": " Night "})))
            .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["name"], "Night");
    let items = format!("/api/v1/users/u1/playlists/{}/items", created["id"].as_str().unwrap());

    for (target, added) in [
        (json!({"type": "movie", "id": "m1"}), 1),
        (json!({"type": "season", "id": "se1"}), 1),
        (json!({"type": "movie", "id": "m1"}), 0),
    ] {
        let (status, body) = call(&app, Method::POST, &items, Some(target)).await;
        assert_eq!((status, body["added"].as_u64()), (StatusCode::OK, Some(added)));
    }
    let (status, _) =
        call(&app, Method::PUT, &format!("{items}/e1/position"), Some(json!({"position": 0})))
            .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let playlist = items.trim_end_matches("/items");
    let (_, detail) = call(&app, Method::GET, playlist, None).await;
    assert_eq!(cards(&detail["items"]), [titled("episode", "e1"), titled("movie", "m1")]);
    assert_eq!(detail["items"][1]["position"], 1);

    let (status, _) = call(&app, Method::DELETE, "/api/v1/users/u1/queue", None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let request = json!({"playlist_id": created["id"]});
    let (_, queued) = call(&app, Method::POST, "/api/v1/users/u1/queue", Some(request)).await;
    assert_eq!(queued["added"], 2);

    let (_, home) = call(&app, Method::GET, "/api/v1/users/u1/continue", None).await;
    assert_eq!(home["queue_next"]["card"]["id"], "e1");
    let first = home["queue_next"]["id"].as_str().unwrap();
    let (status, _) =
        call(&app, Method::DELETE, &format!("/api/v1/users/u1/queue/items/{first}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, queue) = call(&app, Method::GET, "/api/v1/users/u1/queue", None).await;
    assert_eq!(cards(&queue), [titled("movie", "m1")]);
    let (_, home) = call(&app, Method::GET, "/api/v1/users/u1/continue", None).await;
    assert_eq!(home["queue_next"]["card"]["id"], "m1");
}

#[tokio::test]
async fn an_empty_queue_leaves_up_next_without_a_queue_title() {
    let app = app(&Generator::new());
    call(&app, Method::DELETE, "/api/v1/users/u1/queue", None).await;

    let (status, home) = call(&app, Method::GET, "/api/v1/users/u1/continue", None).await;

    assert_eq!(status, StatusCode::OK);
    assert!(home["queue_next"].is_null());
}

#[tokio::test]
async fn another_accounts_playlists_and_queue_are_out_of_reach() {
    let app = app(&Generator::new());

    for (method, uri, body) in [
        (Method::GET, "/api/v1/users/u2/playlists", None),
        (Method::POST, "/api/v1/users/u2/playlists", Some(json!({"name": "x"}))),
        (Method::GET, "/api/v1/users/u2/playlists/pl1", None),
        (Method::PUT, "/api/v1/users/u2/playlists/pl1", Some(json!({"name": "x"}))),
        (Method::DELETE, "/api/v1/users/u2/playlists/pl1", None),
        (
            Method::POST,
            "/api/v1/users/u2/playlists/pl1/items",
            Some(json!({"type": "movie", "id": "m1"})),
        ),
        (Method::DELETE, "/api/v1/users/u2/playlists/pl1/items/m1", None),
        (
            Method::PUT,
            "/api/v1/users/u2/playlists/pl1/items/m1/position",
            Some(json!({"position": 0})),
        ),
        (Method::GET, "/api/v1/users/u2/queue", None),
        (Method::POST, "/api/v1/users/u2/queue", Some(json!({"playlist_id": "pl1"}))),
        (Method::DELETE, "/api/v1/users/u2/queue", None),
        (Method::DELETE, "/api/v1/users/u2/queue/items/1", None),
    ] {
        let (status, _) = call(&app, method, uri, body).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{uri}");
    }
}

#[tokio::test]
async fn a_missing_playlist_or_title_answers_not_found() {
    let app = app(&Generator::new());

    for (method, uri, body) in [
        (Method::PUT, "/api/v1/users/u1/playlists/ghost", Some(json!({"name": "x"}))),
        (Method::DELETE, "/api/v1/users/u1/playlists/ghost", None),
        (Method::DELETE, "/api/v1/users/u1/playlists/ghost/items/m1", None),
        (
            Method::PUT,
            "/api/v1/users/u1/playlists/pl1/items/ghost/position",
            Some(json!({"position": 0})),
        ),
        (
            Method::POST,
            "/api/v1/users/u1/queue",
            Some(json!({"playlist_id": "ghost", "shuffle": true})),
        ),
    ] {
        let (status, _) = call(&app, method, uri, body).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
    }
}

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use tower::ServiceExt;

use api::{AppState, router};
use contracts::Generator;
use domain::catalog::{Movie, MovieId};
use jiff::Timestamp;
use mocks::{MockCatalogRepo, MockUserRepo};
use services::catalog::CatalogServiceImpl;

fn build_app(generator: &Generator) -> Router {
    router(AppState::new(
        generator.auth.clone(),
        generator.catalog.clone(),
        generator.session.clone(),
        generator.library.clone(),
        generator.user.clone(),
        generator.user_library.clone(),
        generator.discovery.clone(),
        generator.job.clone(),
    ))
}

fn empty_app() -> Router {
    // Default rather than new, so the Default impl clippy requires alongside an
    // argument-free new is exercised by something rather than sitting dead.
    let generator = Generator::default();
    router(AppState::new(
        generator.auth.clone(),
        CatalogServiceImpl::new(MockCatalogRepo::new(), MockUserRepo::new()),
        generator.session.clone(),
        generator.library.clone(),
        generator.user.clone(),
        generator.user_library.clone(),
        generator.discovery.clone(),
        generator.job.clone(),
    ))
}

async fn get(app: Router, uri: &str, token: Option<&str>) -> (StatusCode, Value) {
    let mut builder = Request::builder().uri(uri);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = app.oneshot(builder.body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value =
        if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    (status, value)
}

#[tokio::test]
async fn a_random_pick_returns_a_version_to_play() {
    let generator = Generator::new();
    let (status, body) =
        get(build_app(&generator), "/api/v1/movies/random", Some("access:u1")).await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.get("version_id").and_then(Value::as_str).is_some(),
        "the client navigates straight to a version, so that is all it needs back"
    );
}

#[tokio::test]
async fn random_play_needs_a_signed_in_reader() {
    for path in [
        "/api/v1/movies/random",
        "/api/v1/series/random",
        "/api/v1/movies/collections/c1/random",
        "/api/v1/series/s1/random",
        "/api/v1/series/s1/seasons/se1/random",
    ] {
        let generator = Generator::new();
        let (status, _) = get(build_app(&generator), path, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{path} must require a token");
    }
}

#[tokio::test]
async fn an_empty_surface_is_not_found_rather_than_a_broken_player() {
    for path in [
        "/api/v1/movies/random",
        "/api/v1/series/random",
        "/api/v1/movies/collections/c1/random",
        "/api/v1/series/s1/random",
        "/api/v1/series/s1/seasons/se1/random",
    ] {
        let (status, _) = get(empty_app(), path, Some("access:u1")).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path} has nothing to pick");
    }
}

#[tokio::test]
async fn a_movie_whose_id_is_random_does_not_capture_the_random_route() {
    let generator = Generator::new();
    generator.catalog_repo.add_movie(Movie {
        id: MovieId("random".into()),
        title: "Random Harvest".into(),
        sort_title: "random harvest".into(),
        year: None,
        overview: None,
        runtime_minutes: None,
        content_rating: None,
        manually_edited: false,
        added_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
        artwork: Vec::new(),
    });
    let app = build_app(&generator);

    let (status, body) = get(app, "/api/v1/movies/random", Some("access:u1")).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.get("version_id").is_some(), "the static segment must win over the id parameter");
    assert!(
        body.get("title").is_none(),
        "falling through to movie detail would return Random Harvest instead of a pick"
    );
}

#[tokio::test]
async fn a_bad_sort_on_the_random_route_is_rejected_like_any_list() {
    let generator = Generator::new();
    let (status, _) =
        get(build_app(&generator), "/api/v1/movies/random?sort=nonsense", Some("access:u1")).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

use std::time::Duration;

use axum::error_handling::HandleErrorLayer;
use axum::extract::MatchedPath;
use axum::http::{HeaderValue, Method, header};
use axum::middleware::{from_fn, from_fn_with_state};
use axum::response::Redirect;
use axum::routing::{delete, get, post, put};
use axum::{BoxError, Router};
use tower::ServiceBuilder;
use tower::timeout::TimeoutLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};

use domain::diagnostics::{PlaybackPrepAudit, ServerDiagnostics, TranscodeBenchmark};
use domain::job::JobLogStore;
use domain::media::{SubtitleProvider, SubtitleReader, SubtitleStore};
use domain::repository::CatalogRepository;
use domain::session::{DownloadTokens, PlayCommandTokens, StreamSource, StreamTokens};
use tower_http::services::ServeDir;

use crate::handlers::{
    admin, auth, catalog, diagnostics, discovery, download, image, job_log, library, play_on,
    server, sessions, stream, subtitle, trickplay, user_library, users, version, webhook,
};
use crate::middleware::{jwt, timed_out, track_stream_bytes};
use crate::state::{
    AppServices, DiagnosticsState, DownloadState, ImageState, JobLogState, PlayOnState,
    StreamState, SubtitleSearchState, SubtitleState, TrickplayState, WebhookClient, WebhookState,
};

pub fn router<S: AppServices>(state: S) -> Router {
    let public = Router::<S>::new()
        .route("/auth/login", post(auth::login::<S>))
        .route("/auth/refresh", post(auth::refresh::<S>))
        .route("/auth/logout", post(auth::logout::<S>))
        .route("/auth/link", post(auth::link::<S>));

    let protected = Router::<S>::new()
        .route("/auth/link/create", post(auth::create_link::<S>))
        .route("/movies", get(catalog::movies::<S>))
        .route("/movies/random", get(catalog::random_movie::<S>))
        .route(
            "/movies/collections",
            get(catalog::collections::<S>).post(catalog::create_collection::<S>),
        )
        .route(
            "/movies/collections/{id}",
            get(catalog::collection::<S>)
                .put(catalog::update_collection::<S>)
                .delete(catalog::delete_collection::<S>),
        )
        .route("/movies/collections/{id}/random", get(catalog::random_in_collection::<S>))
        .route("/movies/{id}", get(catalog::movie::<S>).put(catalog::edit_movie::<S>))
        .route("/movies/{id}/versions", get(catalog::movie_versions::<S>))
        .route("/movies/{id}/collections", get(catalog::movie_collections::<S>))
        .route("/movies/{id}/refresh", post(catalog::refresh_movie::<S>))
        .route("/series", get(catalog::series::<S>))
        .route("/series/random", get(catalog::random_episode::<S>))
        .route("/series/{id}", get(catalog::series_detail::<S>).put(catalog::edit_series::<S>))
        .route("/series/{id}/random", get(catalog::random_in_series::<S>))
        .route("/series/{id}/refresh", post(catalog::refresh_series::<S>))
        .route("/series/{id}/relink", post(catalog::relink_series::<S>))
        .route("/series/{id}/seasons", get(catalog::seasons::<S>))
        .route("/series/{id}/seasons/{season_id}", get(catalog::season::<S>))
        .route("/series/{id}/seasons/{season_id}/random", get(catalog::random_in_season::<S>))
        .route("/series/{id}/seasons/{season_id}/episodes", get(catalog::episodes::<S>))
        .route(
            "/series/{id}/seasons/{season_id}/episodes/{episode_id}",
            get(catalog::episode::<S>).put(catalog::edit_episode::<S>),
        )
        .route(
            "/series/{id}/seasons/{season_id}/episodes/{episode_id}/versions",
            get(catalog::episode_versions::<S>),
        )
        .route("/versions/{id}", get(catalog::version_detail::<S>).delete(version::delete::<S>))
        .route("/versions/{id}/relink", post(catalog::relink_version::<S>))
        .route("/versions/{id}/jobs", get(version::jobs::<S>))
        .route("/versions/{id}/upscale", post(version::upscale::<S>))
        .route("/versions/{id}/subtitles/transcribe", post(version::transcribe::<S>))
        .route("/versions/{id}/subtitles/{subtitle_id}/translate", post(version::translate::<S>))
        .route("/versions/{id}/subtitles/{subtitle_id}/combine", post(version::combine::<S>))
        .route("/titles/batch", post(catalog::title_cards::<S>))
        .route("/people/batch", post(catalog::people_batch::<S>))
        .route("/people/{id}", get(catalog::person::<S>))
        .route("/people/{id}/refresh", post(catalog::refresh_person::<S>))
        .route("/genres", get(catalog::genres::<S>))
        .route("/server/info", get(server::info))
        .route("/libraries", get(library::libraries::<S>).post(library::create_library::<S>))
        .route(
            "/libraries/{id}",
            get(library::library::<S>)
                .put(library::update_library::<S>)
                .delete(library::delete_library::<S>),
        )
        .route("/libraries/{id}/duplicates", get(library::duplicates::<S>))
        .route("/libraries/{id}/duplicates/{did}/dismiss", post(library::dismiss_duplicate::<S>))
        .route(
            "/libraries/{id}/scan",
            get(library::scan_state::<S>).post(library::trigger_scan::<S>),
        )
        .route("/libraries/{id}/refresh-metadata", post(library::refresh_metadata::<S>))
        .route("/libraries/{id}/unmatched", get(library::unmatched::<S>))
        .route(
            "/libraries/{id}/unmatched/{uid}/candidates",
            get(library::unmatched_candidates::<S>),
        )
        .route("/libraries/{id}/unmatched/{uid}/resolve", post(library::resolve_unmatched::<S>))
        .route("/libraries/{id}/versions", get(library::versions::<S>))
        .route("/admin/jobs", get(admin::jobs::<S>))
        .route("/admin/jobs/{id}", get(admin::job::<S>))
        .route("/admin/jobs/{id}/children", get(admin::job_children::<S>))
        .route("/admin/jobs/{id}/cancel", post(admin::cancel_job::<S>))
        .route("/admin/jobs/{id}/retry", post(admin::retry_job::<S>))
        .route("/admin/versions", get(admin::versions::<S>))
        .route("/admin/movies/{id}", delete(admin::delete_movie::<S>))
        .route("/admin/series/{id}", delete(admin::delete_series::<S>))
        .route("/admin/seasons/{id}", delete(admin::delete_season::<S>))
        .route("/admin/episodes/{id}", delete(admin::delete_episode::<S>))
        .route("/admin/fetch", post(admin::create_fetch::<S>))
        .route("/search", get(discovery::search::<S>))
        .route("/sessions", post(sessions::start::<S>))
        .route("/sessions/{id}", delete(sessions::end::<S>))
        .route("/sessions/{id}/progress", post(sessions::heartbeat::<S>))
        .route("/sessions/{id}/seek", post(sessions::seek::<S>))
        .route("/sessions/{id}/update", post(sessions::update::<S>))
        .route("/users", get(users::list::<S>).post(users::create::<S>))
        .route("/users/activity", get(users::activity::<S>))
        .route("/users/self", get(users::current::<S>))
        .route(
            "/users/{id}",
            get(users::get::<S>).put(users::update_profile::<S>).delete(users::delete::<S>),
        )
        .route("/users/{id}/active", put(users::set_active::<S>))
        .route("/users/{id}/continue", get(discovery::continue_watching::<S>))
        .route("/users/{id}/favorites", get(user_library::favorites::<S>))
        .route(
            "/users/{id}/favorites/{title_id}",
            put(user_library::add_favorite::<S>).delete(user_library::remove_favorite::<S>),
        )
        .route(
            "/users/{id}/history",
            get(user_library::history::<S>).delete(user_library::clear_history::<S>),
        )
        .route("/users/{id}/history/{title_id}", delete(user_library::remove_from_history::<S>))
        .route("/users/{id}/hub", get(discovery::hub::<S>))
        .route(
            "/users/{id}/libraries",
            get(users::library_access::<S>).put(users::set_library_access::<S>),
        )
        .route(
            "/users/{id}/progress/{version}",
            get(user_library::progress::<S>).delete(user_library::clear_progress::<S>),
        )
        .route("/users/{id}/password", put(users::change_password::<S>))
        .route("/users/{id}/state/batch", post(user_library::state_batch::<S>))
        .route("/users/{id}/state/rollup", post(user_library::state_rollup::<S>))
        .route("/users/{id}/sessions", delete(auth::logout_all::<S>))
        .route("/users/{id}/devices", get(auth::devices::<S>))
        .route("/users/{id}/devices/{did}", delete(auth::revoke_device::<S>))
        .route("/users/{id}/tokens", get(auth::tokens::<S>))
        .route("/users/{id}/tokens/{tid}", delete(auth::revoke_token::<S>))
        .route("/users/{id}/link-codes", get(auth::link_codes::<S>))
        .route("/users/{id}/link-codes/{code}", delete(auth::revoke_link_code::<S>))
        .route("/users/{id}/watched/{reference}", put(user_library::set_watched::<S>))
        .route("/users/{id}/watchlist", get(user_library::watchlist::<S>))
        .route(
            "/users/{id}/watchlist/{title_id}",
            put(user_library::add_to_watchlist::<S>)
                .delete(user_library::remove_from_watchlist::<S>),
        )
        .route_layer(from_fn_with_state(state.clone(), jwt::<S>));

    Router::<S>::new().nest("/api/v1", public.merge(protected)).with_state(state)
}

pub fn stream_router<T, G>(state: StreamState<T, G>) -> Router
where
    T: StreamTokens + Send + Sync + 'static,
    G: StreamSource + Send + Sync + 'static,
{
    Router::new()
        .route("/stream/{token}/master.m3u8", get(stream::master::<T, G>))
        .route("/stream/{token}/file", get(stream::file::<T, G>))
        .route("/stream/{token}/{variant}/{file}", get(stream::media::<T, G>))
        .with_state(state)
        .layer(from_fn(track_stream_bytes))
}

pub fn image_router(state: ImageState) -> Router {
    Router::new().route("/images/{artwork_id}/{width}", get(image::image)).with_state(state)
}

pub fn basic_ui_router(dir: &std::path::Path) -> Router {
    if !dir.is_dir() {
        return Router::new();
    }
    Router::new()
        .route("/", get(|| async { Redirect::temporary("/ui/basic/") }))
        .route("/ui", get(|| async { Redirect::temporary("/ui/basic/") }))
        .nest_service("/ui/basic", ServeDir::new(dir))
}

pub fn webhook_router<S: AppServices>(services: S, clients: Vec<WebhookClient>) -> Router {
    if clients.is_empty() {
        return Router::new();
    }
    Router::new()
        .route("/api/v1/webhooks/libraries/{id}/scan", post(webhook::scan::<S>))
        .with_state(WebhookState { services, clients })
}

pub fn job_log_router<S: AppServices, J: JobLogStore + 'static>(
    auth: S,
    state: JobLogState<J>,
) -> Router {
    Router::new()
        .route("/api/v1/admin/jobs/{id}/logs", get(job_log::read::<J>).delete(job_log::wipe::<J>))
        .route_layer(from_fn_with_state(auth, jwt::<S>))
        .with_state(state)
}

pub fn diagnostics_router<S, D, B, C, P>(auth: S, state: DiagnosticsState<D, B, C, P>) -> Router
where
    S: AppServices,
    D: ServerDiagnostics + 'static,
    B: TranscodeBenchmark + 'static,
    C: CatalogRepository + Send + Sync + 'static,
    P: PlaybackPrepAudit + 'static,
{
    Router::new()
        .route("/api/v1/admin/server/capabilities", get(diagnostics::capabilities::<D, B, C, P>))
        .route(
            "/api/v1/admin/server/capabilities/recheck",
            post(diagnostics::recheck::<D, B, C, P>),
        )
        .route(
            "/api/v1/admin/server/benchmark",
            get(diagnostics::benchmark::<D, B, C, P>)
                .post(diagnostics::start_benchmark::<D, B, C, P>),
        )
        .route("/api/v1/admin/server/playback-prep", get(diagnostics::playback_prep::<D, B, C, P>))
        .route_layer(from_fn_with_state(auth, jwt::<S>))
        .with_state(state)
}

pub fn trickplay_router<S: AppServices>(auth: S, state: TrickplayState) -> Router {
    Router::new()
        .route("/api/v1/trickplay/{version_id}/{sheet}", get(trickplay::trickplay))
        .route_layer(from_fn_with_state(auth, jwt::<S>))
        .with_state(state)
}

pub fn download_router<A, C, D>(auth: A, state: DownloadState<A, C, D>) -> Router
where
    A: AppServices,
    C: CatalogRepository + Send + Sync + 'static,
    D: DownloadTokens + Send + Sync + 'static,
{
    let mint = Router::new()
        .route("/api/v1/versions/{id}/download", post(download::link::<A, C, D>))
        .route_layer(from_fn_with_state(auth, jwt::<A>));
    let fetch = Router::new().route("/download/{token}", get(download::file::<A, C, D>));
    mint.merge(fetch).with_state(state)
}

pub fn play_on_router<A, T>(auth: A, state: PlayOnState<A, T>) -> Router
where
    A: AppServices,
    T: PlayCommandTokens + Send + Sync + 'static,
{
    Router::new()
        .route(
            "/api/v1/devices/self/target",
            put(play_on::report_target::<A, T>).delete(play_on::clear_target::<A, T>),
        )
        .route("/api/v1/devices/self/commands/redeem", post(play_on::redeem::<A, T>))
        .route("/api/v1/devices/{did}/commands", post(play_on::mint::<A, T>))
        .route_layer(from_fn_with_state(auth, jwt::<A>))
        .with_state(state)
}

pub fn subtitle_router<A, C, S>(auth: A, state: SubtitleState<C, S>) -> Router
where
    A: AppServices,
    C: CatalogRepository + Send + Sync + 'static,
    S: SubtitleStore + SubtitleReader + Send + Sync + 'static,
{
    Router::new()
        .route(
            "/api/v1/versions/{id}/subtitles/{subtitle_id}",
            get(subtitle::view::<C, S>)
                .put(subtitle::rename::<C, S>)
                .delete(subtitle::delete::<C, S>),
        )
        .route_layer(from_fn_with_state(auth, jwt::<A>))
        .with_state(state)
}

pub fn subtitle_search_router<A, C, S, P>(auth: A, state: SubtitleSearchState<C, S, P>) -> Router
where
    A: AppServices,
    C: CatalogRepository + Send + Sync + 'static,
    S: SubtitleStore + Send + Sync + 'static,
    P: SubtitleProvider + Send + Sync + 'static,
{
    Router::new()
        .route("/api/v1/versions/{id}/subtitles/search", get(subtitle::search::<C, S, P>))
        .route("/api/v1/versions/{id}/subtitles/download", post(subtitle::download::<C, S, P>))
        .route_layer(from_fn_with_state(auth, jwt::<A>))
        .with_state(state)
}

pub fn bounded(router: Router, limit: Duration) -> Router {
    let seconds = limit.as_secs();
    router.layer(
        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(
                move |method: Method, matched: Option<MatchedPath>, _: BoxError| {
                    timed_out(method, matched, seconds)
                },
            ))
            .layer(TimeoutLayer::new(limit)),
    )
}

pub fn cors_layer(origins: &[String]) -> CorsLayer {
    let allowed: Vec<HeaderValue> =
        origins.iter().filter_map(|origin| origin.parse().ok()).collect();
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed))
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT, header::RANGE])
        .expose_headers([
            header::CONTENT_LENGTH,
            header::CONTENT_RANGE,
            header::ACCEPT_RANGES,
            header::ETAG,
        ])
}

#[cfg(test)]
mod bounded_tests {
    use std::convert::Infallible;

    use axum::body::{Body, to_bytes};
    use axum::http::{HeaderMap, Request, StatusCode};
    use futures::{StreamExt, stream};
    use tower::ServiceExt;
    use tracing_test::traced_test;

    use super::*;

    const SHORT: Duration = Duration::from_millis(50);

    async fn answer(router: Router, uri: &str) -> (StatusCode, HeaderMap, String) {
        let request = Request::get(uri).body(Body::empty()).unwrap();
        let (parts, body) = router.oneshot(request).await.unwrap().into_parts();
        let body = to_bytes(body, usize::MAX).await.unwrap();
        (parts.status, parts.headers, String::from_utf8(body.to_vec()).unwrap())
    }

    async fn slow_body() -> Body {
        let chunks = stream::iter(["first ", "second"]).then(async |chunk| {
            tokio::time::sleep(SHORT * 2).await;
            Ok::<_, Infallible>(chunk)
        });
        Body::from_stream(chunks)
    }

    #[traced_test]
    #[tokio::test]
    async fn a_request_with_no_answer_in_time_gets_the_timeout_envelope() {
        let router = Router::new().route("/never/{id}", get(std::future::pending::<()>));

        let (status, _, body) = answer(bounded(router, SHORT), "/never/7").await;

        assert_eq!(status, StatusCode::GATEWAY_TIMEOUT);
        let envelope =
            r#"{"error":{"code":"timeout","message":"the server took too long to answer"}}"#;
        assert_eq!(body, envelope);
        assert!(logs_contain("Request [GET /never/{id}] gave up after [0s] with no answer"));
        assert!(!logs_contain("/never/7"), "the route is logged, never the path it was asked for");
    }

    #[tokio::test]
    async fn a_body_that_streams_past_the_limit_arrives_whole() {
        let router = Router::new().route("/slow", get(slow_body));

        let (status, _, body) = answer(bounded(router, SHORT), "/slow").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "first second");
    }

    #[tokio::test]
    async fn an_answer_in_time_passes_through_untouched_with_or_without_a_limit() {
        let teapot = async || (StatusCode::IM_A_TEAPOT, [("x-kept", "yes")], "brewed");
        for limit in [SHORT, Duration::MAX] {
            let router = Router::new().route("/tea", get(teapot));

            let (status, headers, body) = answer(bounded(router, limit), "/tea").await;

            assert_eq!(status, StatusCode::IM_A_TEAPOT);
            assert_eq!(headers.get("x-kept").unwrap(), "yes");
            assert_eq!(body, "brewed");
        }
    }
}

#[cfg(test)]
mod basic_ui_tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn serves_the_index_and_redirects_the_roots_to_it() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "<h1>basic</h1>").unwrap();
        let router = basic_ui_router(dir.path());

        let served = router
            .clone()
            .oneshot(Request::get("/ui/basic/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(served.status(), StatusCode::OK);

        for root in ["/", "/ui"] {
            let redirected = router
                .clone()
                .oneshot(Request::get(root).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert!(redirected.status().is_redirection());
            assert_eq!(
                redirected.headers().get("location").unwrap().to_str().unwrap(),
                "/ui/basic/"
            );
        }
    }

    #[tokio::test]
    async fn without_a_directory_it_serves_nothing() {
        let router = basic_ui_router(std::path::Path::new("does-not-exist-shadowmask-basic"));
        let response =
            router.oneshot(Request::get("/").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

#[cfg(test)]
mod cors_tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use tower::ServiceExt;

    fn app() -> Router {
        Router::new().route("/api/v1/ping", get(|| async { "pong" })).layer(cors_layer(&[
            "http://localhost:8080".to_owned(),
            "not a valid origin".to_owned(),
        ]))
    }

    #[tokio::test]
    async fn reflects_an_allowed_origin_and_exposes_headers() {
        let response = app()
            .oneshot(
                Request::get("/api/v1/ping")
                    .header("origin", "http://localhost:8080")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("access-control-allow-origin").unwrap(),
            "http://localhost:8080"
        );
        assert!(response.headers().contains_key("access-control-expose-headers"));
    }

    #[tokio::test]
    async fn answers_a_preflight_with_the_allowed_methods() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method(Method::OPTIONS)
                    .uri("/api/v1/ping")
                    .header("origin", "http://localhost:8080")
                    .header("access-control-request-method", "POST")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let methods =
            response.headers().get("access-control-allow-methods").unwrap().to_str().unwrap();
        assert!(methods.contains("POST"));
        assert!(methods.contains("DELETE"));
    }

    #[tokio::test]
    async fn does_not_reflect_a_disallowed_origin() {
        let response = app()
            .oneshot(
                Request::get("/api/v1/ping")
                    .header("origin", "http://evil.example")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(!response.headers().contains_key("access-control-allow-origin"));
    }

    #[tokio::test]
    async fn a_request_that_timed_out_still_carries_the_cors_headers() {
        let hung = Router::new().route("/api/v1/hung", get(std::future::pending::<()>));
        let app = bounded(hung, Duration::from_millis(50))
            .layer(cors_layer(&["http://localhost:8080".to_owned()]));

        let response = app
            .oneshot(
                Request::get("/api/v1/hung")
                    .header("origin", "http://localhost:8080")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);
        assert_eq!(
            response.headers().get("access-control-allow-origin").unwrap(),
            "http://localhost:8080"
        );
    }
}

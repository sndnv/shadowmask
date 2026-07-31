use axum::Router;
use axum::middleware::{from_fn, from_fn_with_state};
use axum::response::Redirect;
use axum::routing::{delete, get, post, put};

use domain::job::JobLogStore;
use domain::media::{SubtitleReader, SubtitleStore};
use domain::repository::CatalogRepository;
use domain::session::{StreamSource, StreamTokens};
use tower_http::services::ServeDir;

use crate::handlers::{
    admin, auth, catalog, discovery, image, job_log, library, server, sessions, stream, subtitle,
    trickplay, user_library, users, webhook,
};
use crate::middleware::{jwt, track_stream_bytes};
use crate::state::{
    AppServices, ImageState, JobLogState, StreamState, SubtitleState, TrickplayState,
    WebhookClient, WebhookState,
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
        .route("/movies/{id}", get(catalog::movie::<S>))
        .route("/movies/{id}/versions", get(catalog::movie_versions::<S>))
        .route("/movies/{id}/refresh", post(catalog::refresh_movie::<S>))
        .route("/series", get(catalog::series::<S>))
        .route("/series/{id}", get(catalog::series_detail::<S>))
        .route("/series/{id}/refresh", post(catalog::refresh_series::<S>))
        .route("/series/{id}/seasons", get(catalog::seasons::<S>))
        .route(
            "/series/{id}/seasons/{season_id}",
            get(catalog::season::<S>),
        )
        .route(
            "/series/{id}/seasons/{season_id}/episodes",
            get(catalog::episodes::<S>),
        )
        .route(
            "/series/{id}/seasons/{season_id}/episodes/{episode_id}",
            get(catalog::episode::<S>),
        )
        .route(
            "/series/{id}/seasons/{season_id}/episodes/{episode_id}/versions",
            get(catalog::episode_versions::<S>),
        )
        .route("/versions/{id}", get(catalog::version_detail::<S>))
        .route("/versions/{id}/relink", post(catalog::relink_version::<S>))
        .route("/titles/batch", post(catalog::title_cards::<S>))
        .route("/people/{id}", get(catalog::person::<S>))
        .route("/genres", get(catalog::genres::<S>))
        .route("/server/info", get(server::info))
        .route(
            "/libraries",
            get(library::libraries::<S>).post(library::create_library::<S>),
        )
        .route(
            "/libraries/{id}",
            get(library::library::<S>)
                .put(library::update_library::<S>)
                .delete(library::delete_library::<S>),
        )
        .route("/libraries/{id}/duplicates", get(library::duplicates::<S>))
        .route(
            "/libraries/{id}/duplicates/{did}/dismiss",
            post(library::dismiss_duplicate::<S>),
        )
        .route(
            "/libraries/{id}/duplicates/{did}/resolve",
            post(library::resolve_duplicate::<S>),
        )
        .route(
            "/libraries/{id}/scan",
            get(library::scan_state::<S>).post(library::trigger_scan::<S>),
        )
        .route("/libraries/{id}/unmatched", get(library::unmatched::<S>))
        .route(
            "/libraries/{id}/unmatched/{uid}/candidates",
            get(library::unmatched_candidates::<S>),
        )
        .route(
            "/libraries/{id}/unmatched/{uid}/resolve",
            post(library::resolve_unmatched::<S>),
        )
        .route("/libraries/{id}/versions", get(library::versions::<S>))
        .route("/admin/jobs", get(admin::jobs::<S>))
        .route("/admin/jobs/{id}", get(admin::job::<S>))
        .route("/admin/jobs/{id}/cancel", post(admin::cancel_job::<S>))
        .route("/admin/versions", get(admin::versions::<S>))
        .route(
            "/admin/versions/{id}/transcribe",
            post(admin::transcribe_version::<S>),
        )
        .route(
            "/admin/versions/{id}/translate",
            post(admin::translate_version::<S>),
        )
        .route(
            "/admin/versions/{id}/upscale",
            post(admin::upscale_version::<S>),
        )
        .route(
            "/admin/versions/{id}/subtitles/combine",
            post(admin::combine_subtitles::<S>),
        )
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
            get(users::get::<S>)
                .put(users::update_profile::<S>)
                .delete(users::delete::<S>),
        )
        .route(
            "/users/{id}/continue",
            get(discovery::continue_watching::<S>),
        )
        .route("/users/{id}/favorites", get(user_library::favorites::<S>))
        .route(
            "/users/{id}/favorites/{title_id}",
            put(user_library::add_favorite::<S>).delete(user_library::remove_favorite::<S>),
        )
        .route("/users/{id}/history", get(user_library::history::<S>))
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
        .route(
            "/users/{id}/state/batch",
            post(user_library::state_batch::<S>),
        )
        .route(
            "/users/{id}/state/rollup",
            post(user_library::state_rollup::<S>),
        )
        .route("/users/{id}/sessions", delete(auth::logout_all::<S>))
        .route("/users/{id}/devices", get(auth::devices::<S>))
        .route(
            "/users/{id}/devices/{did}",
            delete(auth::revoke_device::<S>),
        )
        .route("/users/{id}/tokens", get(auth::tokens::<S>))
        .route("/users/{id}/tokens/{tid}", delete(auth::revoke_token::<S>))
        .route("/users/{id}/link-codes", get(auth::link_codes::<S>))
        .route(
            "/users/{id}/link-codes/{code}",
            delete(auth::revoke_link_code::<S>),
        )
        .route(
            "/users/{id}/watched/{reference}",
            put(user_library::set_watched::<S>),
        )
        .route("/users/{id}/watchlist", get(user_library::watchlist::<S>))
        .route(
            "/users/{id}/watchlist/{title_id}",
            put(user_library::add_to_watchlist::<S>)
                .delete(user_library::remove_from_watchlist::<S>),
        )
        .route_layer(from_fn_with_state(state.clone(), jwt::<S>));

    Router::<S>::new()
        .nest("/api/v1", public.merge(protected))
        .with_state(state)
}

pub fn stream_router<T, G>(state: StreamState<T, G>) -> Router
where
    T: StreamTokens + Send + Sync + 'static,
    G: StreamSource + Send + Sync + 'static,
{
    Router::new()
        .route("/stream/{token}/master.m3u8", get(stream::master::<T, G>))
        .route("/stream/{token}/file", get(stream::file::<T, G>))
        .route(
            "/stream/{token}/{variant}/{file}",
            get(stream::media::<T, G>),
        )
        .with_state(state)
        .layer(from_fn(track_stream_bytes))
}

pub fn image_router(state: ImageState) -> Router {
    Router::new()
        .route("/images/{artwork_id}/{width}", get(image::image))
        .with_state(state)
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
        .route(
            "/api/v1/webhooks/libraries/{id}/scan",
            post(webhook::scan::<S>),
        )
        .with_state(WebhookState { services, clients })
}

pub fn job_log_router<S: AppServices, J: JobLogStore + 'static>(
    auth: S,
    state: JobLogState<J>,
) -> Router {
    Router::new()
        .route(
            "/api/v1/admin/jobs/{id}/logs",
            get(job_log::read::<J>).delete(job_log::wipe::<J>),
        )
        .route_layer(from_fn_with_state(auth, jwt::<S>))
        .with_state(state)
}

pub fn trickplay_router<S: AppServices>(auth: S, state: TrickplayState) -> Router {
    Router::new()
        .route(
            "/api/v1/trickplay/{version_id}/{sheet}",
            get(trickplay::trickplay),
        )
        .route_layer(from_fn_with_state(auth, jwt::<S>))
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
            "/api/v1/admin/versions/{id}/subtitles/{subtitle_id}",
            get(subtitle::view::<C, S>).delete(subtitle::delete::<C, S>),
        )
        .route_layer(from_fn_with_state(auth, jwt::<A>))
        .with_state(state)
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
                redirected
                    .headers()
                    .get("location")
                    .unwrap()
                    .to_str()
                    .unwrap(),
                "/ui/basic/"
            );
        }
    }

    #[tokio::test]
    async fn without_a_directory_it_serves_nothing() {
        let router = basic_ui_router(std::path::Path::new("does-not-exist-shadowmask-basic"));
        let response = router
            .oneshot(Request::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

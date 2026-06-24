use axum::Router;
use axum::middleware::from_fn_with_state;
use axum::routing::{delete, get, post, put};

use domain::session::{StreamSource, StreamTokens};

use crate::handlers::{auth, catalog, discovery, library, sessions, stream, user_library, users};
use crate::middleware::jwt;
use crate::state::{AppServices, StreamState};

pub fn router<S: AppServices>(state: S) -> Router {
    let public = Router::<S>::new()
        .route("/auth/login", post(auth::login::<S>))
        .route("/auth/refresh", post(auth::refresh::<S>))
        .route("/auth/link", post(auth::link::<S>));

    let protected = Router::<S>::new()
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
        .route("/series", get(catalog::series::<S>))
        .route("/series/{id}", get(catalog::series_detail::<S>))
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
        .route("/libraries", get(library::libraries::<S>))
        .route("/libraries/{id}", get(library::library::<S>))
        .route("/libraries/{id}/duplicates", get(library::duplicates::<S>))
        .route(
            "/libraries/{id}/scan",
            get(library::scan_state::<S>).post(library::trigger_scan::<S>),
        )
        .route("/libraries/{id}/unmatched", get(library::unmatched::<S>))
        .route("/libraries/{id}/versions", get(library::versions::<S>))
        .route("/search", get(discovery::search::<S>))
        .route("/sessions", post(sessions::start::<S>))
        .route("/sessions/{id}", delete(sessions::end::<S>))
        .route("/sessions/{id}/progress", post(sessions::heartbeat::<S>))
        .route("/sessions/{id}/seek", post(sessions::seek::<S>))
        .route("/sessions/{id}/update", post(sessions::update::<S>))
        .route("/users", get(users::list::<S>).post(users::create::<S>))
        .route("/users/activity", get(users::activity::<S>))
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
            get(user_library::progress::<S>),
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
}

use serde_json::{Value, json};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Token {
    Admin,
    User,
    Anon,
}

impl Token {
    pub fn header(self) -> Option<&'static str> {
        match self {
            Token::Admin => Some("access:admin"),
            Token::User => Some("access:u1"),
            Token::Anon => None,
        }
    }
}

pub struct EndpointCase {
    pub name: &'static str,
    pub method: &'static str,
    pub path: &'static str,
    pub token: Token,
    pub body: Option<Value>,
}

fn case(
    name: &'static str,
    method: &'static str,
    path: &'static str,
    token: Token,
    body: Option<Value>,
) -> EndpointCase {
    EndpointCase {
        name,
        method,
        path,
        token,
        body,
    }
}

pub fn requests() -> Vec<EndpointCase> {
    vec![
        case(
            "auth_login",
            "POST",
            "/api/v1/auth/login",
            Token::Anon,
            Some(json!({"username": "admin", "password": "pw"})),
        ),
        case(
            "auth_refresh",
            "POST",
            "/api/v1/auth/refresh",
            Token::Anon,
            Some(json!({"refresh_token": "refresh:u1"})),
        ),
        case(
            "auth_link",
            "POST",
            "/api/v1/auth/link",
            Token::Anon,
            Some(json!({"code": "CODE", "device": {"name": "Roku", "platform": "roku"}})),
        ),
        case("movies_list", "GET", "/api/v1/movies", Token::User, None),
        case(
            "movies_collections",
            "GET",
            "/api/v1/movies/collections",
            Token::User,
            None,
        ),
        case(
            "movies_create_collection",
            "POST",
            "/api/v1/movies/collections",
            Token::Admin,
            Some(json!({"name": "New", "overview": null, "movies": ["m1"]})),
        ),
        case(
            "movies_collection_detail",
            "GET",
            "/api/v1/movies/collections/c1",
            Token::User,
            None,
        ),
        case(
            "movies_update_collection",
            "PUT",
            "/api/v1/movies/collections/c1",
            Token::Admin,
            Some(json!({"name": "Renamed", "overview": "now", "movies": []})),
        ),
        case(
            "movies_delete_collection",
            "DELETE",
            "/api/v1/movies/collections/c1",
            Token::Admin,
            None,
        ),
        case(
            "movie_detail",
            "GET",
            "/api/v1/movies/m1",
            Token::User,
            None,
        ),
        case(
            "movie_versions",
            "GET",
            "/api/v1/movies/m1/versions",
            Token::User,
            None,
        ),
        case("series_list", "GET", "/api/v1/series", Token::User, None),
        case(
            "series_detail",
            "GET",
            "/api/v1/series/s1",
            Token::User,
            None,
        ),
        case(
            "series_seasons",
            "GET",
            "/api/v1/series/s1/seasons",
            Token::User,
            None,
        ),
        case(
            "series_season_detail",
            "GET",
            "/api/v1/series/s1/seasons/se1",
            Token::User,
            None,
        ),
        case(
            "series_episodes",
            "GET",
            "/api/v1/series/s1/seasons/se1/episodes",
            Token::User,
            None,
        ),
        case(
            "series_episode_detail",
            "GET",
            "/api/v1/series/s1/seasons/se1/episodes/e1",
            Token::User,
            None,
        ),
        case(
            "series_episode_versions",
            "GET",
            "/api/v1/series/s1/seasons/se1/episodes/e1/versions",
            Token::User,
            None,
        ),
        case(
            "libraries_list",
            "GET",
            "/api/v1/libraries",
            Token::User,
            None,
        ),
        case(
            "library_detail",
            "GET",
            "/api/v1/libraries/lib1",
            Token::User,
            None,
        ),
        case(
            "library_duplicates",
            "GET",
            "/api/v1/libraries/lib1/duplicates",
            Token::Admin,
            None,
        ),
        case(
            "library_scan_state",
            "GET",
            "/api/v1/libraries/lib1/scan",
            Token::Admin,
            None,
        ),
        case(
            "library_trigger_scan",
            "POST",
            "/api/v1/libraries/lib1/scan",
            Token::Admin,
            None,
        ),
        case(
            "library_unmatched",
            "GET",
            "/api/v1/libraries/lib1/unmatched",
            Token::Admin,
            None,
        ),
        case(
            "library_versions",
            "GET",
            "/api/v1/libraries/lib1/versions",
            Token::Admin,
            None,
        ),
        case("search", "GET", "/api/v1/search?q=alpha", Token::User, None),
        case(
            "session_start",
            "POST",
            "/api/v1/sessions",
            Token::User,
            Some(json!({
                "version_id": "v1",
                "capabilities": {"platform": "web", "profile_version": 1, "max_bitrate": null},
                "audio_track": 0,
                "subtitle": {"track": {"type": "embedded", "index": 1}, "offset_ms": 500}
            })),
        ),
        case(
            "session_progress",
            "POST",
            "/api/v1/sessions/{session}/progress",
            Token::User,
            Some(json!({"position_ms": 1000, "state": "paused"})),
        ),
        case(
            "session_seek",
            "POST",
            "/api/v1/sessions/{session}/seek",
            Token::User,
            Some(json!({"position_ms": 2000})),
        ),
        case(
            "session_update",
            "POST",
            "/api/v1/sessions/{session}/update",
            Token::User,
            Some(json!({"audio_track": 2, "subtitle": {"action": "keep"}})),
        ),
        case(
            "session_end",
            "DELETE",
            "/api/v1/sessions/{session}",
            Token::User,
            None,
        ),
        case("users_list", "GET", "/api/v1/users", Token::Admin, None),
        case(
            "users_create",
            "POST",
            "/api/v1/users",
            Token::Admin,
            Some(json!({"username": "newbie", "password": "pw", "role": "player"})),
        ),
        case(
            "users_activity",
            "GET",
            "/api/v1/users/activity",
            Token::Admin,
            None,
        ),
        case(
            "user_detail",
            "GET",
            "/api/v1/users/{user}",
            Token::Admin,
            None,
        ),
        case(
            "user_update_profile",
            "PUT",
            "/api/v1/users/{user}",
            Token::Admin,
            Some(json!({
                "preferred_audio": ["en"],
                "preferred_subtitle": ["fr"],
                "max_content_rating": {"system": "MPAA", "code": "R"},
                "concurrent_stream_limit": 2,
                "bitrate_cap": 8000000
            })),
        ),
        case(
            "user_delete",
            "DELETE",
            "/api/v1/users/{user}",
            Token::Admin,
            None,
        ),
        case(
            "user_continue",
            "GET",
            "/api/v1/users/u1/continue",
            Token::User,
            None,
        ),
        case(
            "user_favorites",
            "GET",
            "/api/v1/users/u1/favorites",
            Token::User,
            None,
        ),
        case(
            "user_add_favorite",
            "PUT",
            "/api/v1/users/u1/favorites/m1",
            Token::User,
            Some(json!({"type": "movie"})),
        ),
        case(
            "user_remove_favorite",
            "DELETE",
            "/api/v1/users/u1/favorites/m1",
            Token::User,
            None,
        ),
        case(
            "user_history",
            "GET",
            "/api/v1/users/u1/history",
            Token::User,
            None,
        ),
        case("user_hub", "GET", "/api/v1/users/u1/hub", Token::User, None),
        case(
            "user_library_access",
            "GET",
            "/api/v1/users/{user}/libraries",
            Token::Admin,
            None,
        ),
        case(
            "user_set_library_access",
            "PUT",
            "/api/v1/users/{user}/libraries",
            Token::Admin,
            Some(json!({"libraries": ["lib1"]})),
        ),
        case(
            "user_progress",
            "GET",
            "/api/v1/users/u1/progress/v1",
            Token::User,
            None,
        ),
        case(
            "user_watchlist",
            "GET",
            "/api/v1/users/u1/watchlist",
            Token::User,
            None,
        ),
        case(
            "user_add_watchlist",
            "PUT",
            "/api/v1/users/u1/watchlist/m1",
            Token::User,
            Some(json!({"type": "movie"})),
        ),
        case(
            "user_remove_watchlist",
            "DELETE",
            "/api/v1/users/u1/watchlist/m1",
            Token::User,
            None,
        ),
    ]
}

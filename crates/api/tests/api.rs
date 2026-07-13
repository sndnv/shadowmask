use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use jiff::Timestamp;
use serde_json::{Value, json};
use tower::ServiceExt;

use api::{AppState, router};
use domain::catalog::*;
use domain::common::Quality;
use domain::discovery::*;
use domain::library::*;
use domain::metadata::*;
use domain::playback::*;
use domain::user::{IssuedToken, Role, UserId};
use services::mock::*;

const ADMIN: &str = "access:admin";
const USER: &str = "access:u1";

struct Ctx {
    auth: MockAuthService,
    catalog: MockCatalogService,
    session: MockSessionService,
    library: MockLibraryService,
    user: MockUserService,
    user_library: MockUserLibraryService,
    discovery: MockDiscoveryService,
}

impl Ctx {
    fn new() -> Self {
        let auth = MockAuthService::new();
        auth.add_account("admin", "pw", UserId("admin".into()), Role::Admin);
        auth.add_account("user", "pw", UserId("u1".into()), Role::User);
        Ctx {
            auth,
            catalog: MockCatalogService::new(),
            session: MockSessionService::new(),
            library: MockLibraryService::new(),
            user: MockUserService::new(),
            user_library: MockUserLibraryService::new(),
            discovery: MockDiscoveryService::new(),
        }
    }

    fn app(&self) -> Router {
        router(AppState::new(
            self.auth.clone(),
            self.catalog.clone(),
            self.session.clone(),
            self.library.clone(),
            self.user.clone(),
            self.user_library.clone(),
            self.discovery.clone(),
        ))
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

fn movie(id: &str) -> Movie {
    Movie {
        id: MovieId(id.into()),
        title: format!("Alpha {id}"),
        year: Some(2020),
        overview: Some("overview".into()),
        runtime_minutes: Some(100),
        content_rating: Some(ContentRating {
            system: "MPAA".into(),
            code: "PG-13".into(),
        }),
        added_at: Timestamp::now(),
        artwork: Vec::new(),
    }
}

fn series(id: &str) -> Series {
    Series {
        id: SeriesId(id.into()),
        title: format!("Alpha {id}"),
        year: Some(2019),
        overview: None,
        content_rating: Some(ContentRating {
            system: "TV".into(),
            code: "TV-14".into(),
        }),
        added_at: Timestamp::now(),
        artwork: Vec::new(),
    }
}

fn season(id: &str, series: &str) -> Season {
    Season {
        id: SeasonId(id.into()),
        series: SeriesId(series.into()),
        number: 1,
        title: Some("Season 1".into()),
        overview: None,
        artwork: Vec::new(),
    }
}

fn episode(id: &str, season: &str) -> Episode {
    Episode {
        id: EpisodeId(id.into()),
        season: SeasonId(season.into()),
        number: 1,
        title: format!("Alpha {id}"),
        overview: None,
        runtime_minutes: Some(42),
        air_date: Some(Timestamp::now()),
        added_at: Timestamp::now(),
        artwork: Vec::new(),
    }
}

fn version(id: &str, title: TitleId, lib: &str, quality: Quality) -> Version {
    Version {
        id: VersionId(id.into()),
        title,
        library: LibraryId(lib.into()),
        quality,
        container: "mkv".into(),
        path: format!("/media/{id}.mkv"),
        size_bytes: 1,
        duration_ms: 1000,
        edition: None,
    }
}

fn library(id: &str) -> Library {
    Library {
        id: LibraryId(id.into()),
        name: format!("Lib {id}"),
        kind: LibraryKind::Movie,
        roots: vec!["/media".into()],
        watcher: WatcherStrategy::Manual,
        scan_schedule: Some("0 0 * * *".into()),
        metadata_sources: vec!["tmdb".into()],
    }
}

#[tokio::test]
async fn auth_endpoints() {
    let ctx = Ctx::new();
    ctx.auth.add_link_code(
        "CODE",
        IssuedToken {
            token: "player-token".into(),
            expires_at: None,
        },
    );

    let (status, body) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/auth/login",
        None,
        Some(json!({"username": "admin", "password": "pw"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["access_token"], "access:admin");

    let (status, _) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/auth/login",
        None,
        Some(json!({"username": "admin", "password": "wrong"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, body) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/auth/refresh",
        None,
        Some(json!({"refresh_token": "refresh:u1"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["access_token"], "access:u1");

    let (status, body) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/auth/link",
        None,
        Some(json!({"code": "CODE", "device": {"name": "Roku", "platform": "roku"}})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["token"], "player-token");
}

#[tokio::test]
async fn auth_and_rbac_guards() {
    let ctx = Ctx::new();

    // Missing bearer token.
    let (status, _) = call(ctx.app(), Method::GET, "/api/v1/movies", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Malformed authorization header (no "Bearer " prefix).
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/movies")
        .header(header::AUTHORIZATION, "Token nope")
        .body(Body::empty())
        .unwrap();
    let response = ctx.app().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Unknown token.
    let (status, _) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/movies",
        Some("garbage"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Non-admin hitting an admin-only route.
    let (status, _) = call(ctx.app(), Method::GET, "/api/v1/users", Some(USER), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Admin allowed.
    let (status, _) = call(ctx.app(), Method::GET, "/api/v1/users", Some(ADMIN), None).await;
    assert_eq!(status, StatusCode::OK);

    // Non-admin acting on another user's resource.
    let (status, _) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/users/u2/watchlist",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn catalog_routes() {
    let ctx = Ctx::new();
    ctx.catalog.add_movie(movie("m1"));
    ctx.catalog.add_series(series("s1"));
    ctx.catalog.add_season(season("se1", "s1"));
    ctx.catalog.add_episode(episode("e1", "se1"));
    ctx.catalog.add_collection(Collection {
        id: CollectionId("c1".into()),
        name: "Saga".into(),
        overview: Some("epic".into()),
        movies: vec![MovieId("m1".into())],
        artwork: Vec::new(),
    });
    for (vid, q) in [
        ("v1", Quality::Sd),
        ("v2", Quality::Hd),
        ("v3", Quality::Fhd),
        ("v4", Quality::Uhd),
    ] {
        ctx.catalog.add_version(version(
            vid,
            TitleId::Movie(MovieId("m1".into())),
            "lib1",
            q,
        ));
    }
    ctx.catalog.add_version(version(
        "ev1",
        TitleId::Episode(EpisodeId("e1".into())),
        "lib1",
        Quality::Hd,
    ));

    let cases = [
        ("/api/v1/movies", StatusCode::OK),
        ("/api/v1/movies/m1", StatusCode::OK),
        ("/api/v1/movies/m1/versions", StatusCode::OK),
        ("/api/v1/movies/collections", StatusCode::OK),
        ("/api/v1/movies/collections/c1", StatusCode::OK),
        ("/api/v1/series", StatusCode::OK),
        ("/api/v1/series/s1", StatusCode::OK),
        ("/api/v1/series/s1/seasons", StatusCode::OK),
        ("/api/v1/series/s1/seasons/se1", StatusCode::OK),
        ("/api/v1/series/s1/seasons/se1/episodes", StatusCode::OK),
        ("/api/v1/series/s1/seasons/se1/episodes/e1", StatusCode::OK),
        (
            "/api/v1/series/s1/seasons/se1/episodes/e1/versions",
            StatusCode::OK,
        ),
    ];
    for (uri, expected) in cases {
        let (status, _) = call(ctx.app(), Method::GET, uri, Some(USER), None).await;
        assert_eq!(status, expected, "GET {uri}");
    }

    // A not-found exercises the failure logging path.
    let (status, _) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/movies/ghost",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Admin collection management.
    let (status, body) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/movies/collections",
        Some(ADMIN),
        Some(json!({"name": "New", "overview": null, "movies": ["m1"]})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let cid = body["id"].as_str().unwrap().to_string();

    let (status, _) = call(
        ctx.app(),
        Method::PUT,
        &format!("/api/v1/movies/collections/{cid}"),
        Some(ADMIN),
        Some(json!({"name": "Renamed", "overview": "now", "movies": []})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = call(
        ctx.app(),
        Method::DELETE,
        &format!("/api/v1/movies/collections/{cid}"),
        Some(ADMIN),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn library_routes() {
    let ctx = Ctx::new();
    let id = LibraryId("lib1".into());
    ctx.library.add_library(library("lib1"));
    ctx.library.add_unmatched(
        &id,
        UnmatchedFile {
            id: UnmatchedFileId("uf1".into()),
            library: id.clone(),
            path: "/media/x.mkv".into(),
            candidates: vec![MatchCandidate {
                title: TitleId::Movie(MovieId("m1".into())),
                confidence: 0.9,
                label: "Alpha".into(),
            }],
        },
    );
    ctx.library.add_duplicate(
        &id,
        DuplicateCandidate {
            id: DuplicateCandidateId("d1".into()),
            title: TitleId::Episode(EpisodeId("e1".into())),
            paths: vec!["/a.mkv".into(), "/b.mkv".into()],
        },
    );
    ctx.catalog.add_version(version(
        "v1",
        TitleId::Movie(MovieId("m1".into())),
        "lib1",
        Quality::Hd,
    ));

    // Browsing libraries is allowed for any authenticated user.
    let (status, _) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/libraries",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/libraries/lib1",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Admin-only sub-resources.
    let admin_gets = [
        "/api/v1/libraries/lib1/scan",
        "/api/v1/libraries/lib1/unmatched",
        "/api/v1/libraries/lib1/duplicates",
        "/api/v1/libraries/lib1/versions",
    ];
    for uri in admin_gets {
        let (status, _) = call(ctx.app(), Method::GET, uri, Some(ADMIN), None).await;
        assert_eq!(status, StatusCode::OK, "GET {uri}");
    }
    // Scan state is admin-only.
    let (status, _) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/libraries/lib1/scan",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Trigger a scan, then confirm the state flips to running.
    let (status, _) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/libraries/lib1/scan",
        Some(ADMIN),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let (status, body) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/libraries/lib1/scan",
        Some(ADMIN),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "running");
}

#[tokio::test]
async fn session_routes() {
    let ctx = Ctx::new();

    let (status, body) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/sessions",
        Some(USER),
        Some(json!({
            "version_id": "v1",
            "capabilities": {"platform": "web", "profile_version": 1, "max_bitrate": null},
            "audio_track": 0,
            "subtitle": {"track": {"type": "embedded", "index": 1}, "offset_ms": 500}
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let sid = body["session_id"].as_str().unwrap().to_string();

    let (status, _) = call(
        ctx.app(),
        Method::POST,
        &format!("/api/v1/sessions/{sid}/progress"),
        Some(USER),
        Some(json!({"position_ms": 1000, "state": "paused"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = call(
        ctx.app(),
        Method::POST,
        &format!("/api/v1/sessions/{sid}/seek"),
        Some(USER),
        Some(json!({"position_ms": 2000})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    for change in [
        json!({"action": "keep"}),
        json!({"action": "set", "track": {"type": "file", "id": "sf1"}}),
        json!({"action": "disable"}),
    ] {
        let (status, _) = call(
            ctx.app(),
            Method::POST,
            &format!("/api/v1/sessions/{sid}/update"),
            Some(USER),
            Some(json!({"audio_track": 2, "subtitle": change})),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    // subtitle omitted → defaults to keep.
    let (status, _) = call(
        ctx.app(),
        Method::POST,
        &format!("/api/v1/sessions/{sid}/update"),
        Some(USER),
        Some(json!({"audio_track": 3})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Admin can see the active session.
    let (status, body) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/users/activity",
        Some(ADMIN),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["items"].as_array().unwrap().len(), 1);

    let (status, _) = call(
        ctx.app(),
        Method::DELETE,
        &format!("/api/v1/sessions/{sid}"),
        Some(USER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn session_failure_paths() {
    let ctx = Ctx::new();

    // Mutating an unknown session → 404 session_not_found on every route.
    let unknown = "/api/v1/sessions/does-not-exist";
    let cases: [(Method, String, Option<Value>); 4] = [
        (
            Method::POST,
            format!("{unknown}/progress"),
            Some(json!({"position_ms": 1, "state": "playing"})),
        ),
        (
            Method::POST,
            format!("{unknown}/seek"),
            Some(json!({"position_ms": 1})),
        ),
        (
            Method::POST,
            format!("{unknown}/update"),
            Some(json!({"audio_track": 1, "subtitle": {"action": "keep"}})),
        ),
        (Method::DELETE, unknown.to_string(), None),
    ];
    for (method, uri, body) in cases {
        let (status, body) = call(ctx.app(), method, &uri, Some(USER), body).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"]["code"], "session_not_found");
    }

    // Concurrent-stream limit exceeded → 409 listing the user's active sessions.
    ctx.session.set_concurrent_limit(1);
    let start = json!({
        "version_id": "v1",
        "capabilities": {"platform": "web", "profile_version": 1},
        "audio_track": 0
    });
    let (status, _) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/sessions",
        Some(USER),
        Some(start.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, body) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/sessions",
        Some(USER),
        Some(start),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "concurrent_limit");
    assert_eq!(body["active"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn user_routes() {
    let ctx = Ctx::new();

    let (status, body) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/users",
        Some(ADMIN),
        Some(json!({"username": "newbie", "password": "pw", "role": "player"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let uid = body["id"].as_str().unwrap().to_string();

    let (status, _) = call(ctx.app(), Method::GET, "/api/v1/users", Some(ADMIN), None).await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = call(
        ctx.app(),
        Method::GET,
        &format!("/api/v1/users/{uid}"),
        Some(ADMIN),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Self access (Ok branch of admin-or-self); no user entity for u1 so it 404s.
    let (status, _) = call(ctx.app(), Method::GET, "/api/v1/users/u1", Some(USER), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = call(
        ctx.app(),
        Method::PUT,
        &format!("/api/v1/users/{uid}"),
        Some(ADMIN),
        Some(json!({
            "preferred_audio": ["en"],
            "preferred_subtitle": ["fr"],
            "max_content_rating": {"system": "MPAA", "code": "R"},
            "concurrent_stream_limit": 2,
            "bitrate_cap": 8000000
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = call(
        ctx.app(),
        Method::PUT,
        &format!("/api/v1/users/{uid}/libraries"),
        Some(ADMIN),
        Some(json!({"libraries": ["lib1"]})),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, body) = call(
        ctx.app(),
        Method::GET,
        &format!("/api/v1/users/{uid}/libraries"),
        Some(ADMIN),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);

    let (status, _) = call(
        ctx.app(),
        Method::DELETE,
        &format!("/api/v1/users/{uid}"),
        Some(ADMIN),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn user_library_routes() {
    let ctx = Ctx::new();
    let u1 = UserId("u1".into());
    ctx.user_library.add_history(WatchHistory {
        user: u1.clone(),
        title: TitleId::Movie(MovieId("m1".into())),
        watched: true,
        play_count: 2,
        last_watched_at: Some(Timestamp::now()),
        completed: false,
    });
    ctx.user_library.set_progress(PlaybackProgress {
        user: u1.clone(),
        version: VersionId("v1".into()),
        position_ms: 1234,
        updated_at: Timestamp::now(),
    });

    // Watchlist add (movie + episode), list, remove.
    let (status, _) = call(
        ctx.app(),
        Method::PUT,
        "/api/v1/users/u1/watchlist/m1",
        Some(USER),
        Some(json!({"type": "movie"})),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        ctx.app(),
        Method::PUT,
        "/api/v1/users/u1/watchlist/e1",
        Some(USER),
        Some(json!({"type": "episode"})),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, body) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/users/u1/watchlist",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 2);
    let (status, _) = call(
        ctx.app(),
        Method::DELETE,
        "/api/v1/users/u1/watchlist/m1",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Favorites add, list, remove.
    let (status, _) = call(
        ctx.app(),
        Method::PUT,
        "/api/v1/users/u1/favorites/m1",
        Some(USER),
        Some(json!({"type": "movie"})),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/users/u1/favorites",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        ctx.app(),
        Method::DELETE,
        "/api/v1/users/u1/favorites/m1",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // History + progress.
    let (status, body) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/users/u1/history",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["items"].as_array().unwrap().len(), 1);

    let (status, body) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/users/u1/progress/v1",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["position_ms"], 1234);

    // Progress for an unknown version returns null.
    let (status, body) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/users/u1/progress/ghost",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, Value::Null);
}

#[tokio::test]
async fn discovery_routes() {
    let ctx = Ctx::new();
    ctx.discovery
        .add_search_result(SearchResult::Movie(movie("m1")));
    ctx.discovery
        .add_search_result(SearchResult::Series(series("s1")));
    ctx.discovery
        .add_search_result(SearchResult::Episode(episode("e1", "se1")));
    ctx.discovery
        .add_search_result(SearchResult::Person(Person {
            id: PersonId("p1".into()),
            name: "Alpha Person".into(),
        }));
    ctx.discovery.add_continue_watching(
        &UserId("u1".into()),
        ContinueWatchingItem {
            progress: PlaybackProgress {
                user: UserId("u1".into()),
                version: VersionId("v1".into()),
                position_ms: 10,
                updated_at: Timestamp::now(),
            },
            card: ResumeCard {
                title: TitleId::Movie(MovieId("m1".into())),
                display_title: "Alpha m1".into(),
                artwork: Vec::new(),
                duration_ms: 1000,
                progress_percent: 1,
            },
        },
    );
    ctx.discovery
        .add_next_episode(&UserId("u1".into()), episode("e1", "se1"));
    ctx.discovery
        .add_next_movie(&UserId("u1".into()), movie("m2"));
    ctx.discovery.add_hub(Hub {
        id: "recent".into(),
        title: "Recently Added".into(),
        items: vec![HubItem::Movie(movie("m1")), HubItem::Series(series("s1"))],
    });

    let (status, body) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/search?q=alpha",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["items"].as_array().unwrap().len(), 4);

    // Start a session so the continue payload includes "now playing".
    let (status, _) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/sessions",
        Some(USER),
        Some(json!({
            "version_id": "v1",
            "capabilities": {"platform": "web", "profile_version": 1, "max_bitrate": null},
            "audio_track": 0,
            "subtitle": null
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, body) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/users/u1/continue",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["now_playing"].as_array().unwrap().len(), 1);
    assert_eq!(body["in_progress"].as_array().unwrap().len(), 1);
    assert_eq!(body["next_episodes"].as_array().unwrap().len(), 1);
    assert_eq!(body["next_movies"].as_array().unwrap().len(), 1);

    let (status, body) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/users/u1/hub",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);
}

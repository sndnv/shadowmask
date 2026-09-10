use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use jiff::Timestamp;
use serde_json::{Value, json};
use tower::ServiceExt;

use api::{AppState, WebhookClient, router, webhook_router};
use domain::catalog::*;
use domain::common::LanguageCode;
use domain::common::Quality;
use domain::discovery::*;
use domain::library::*;
use domain::media::{AudioTrack, SubtitleFile, SubtitleFileId, SubtitleFormat, SubtitleSource};
use domain::metadata::*;
use domain::playback::*;
use domain::repository::CatalogRepository;
use domain::user::{IssuedToken, Role, User, UserId};
use mocks::*;
use services::catalog::CatalogServiceImpl;
use services::discovery::DiscoveryServiceImpl;
use services::job::JobServiceImpl;
use services::library::LibraryServiceImpl;
use services::user::UserServiceImpl;
use services::user_library::UserLibraryServiceImpl;
use std::sync::Arc;

const ADMIN: &str = "access:admin";
const USER: &str = "access:u1";

type CatalogSvc = CatalogServiceImpl<MockCatalogRepo, MockUserRepo>;
type UserSvc = UserServiceImpl<MockUserRepo, MockAuthTokenRepo, MockUserDataStore>;
type UserLibrarySvc =
    UserLibraryServiceImpl<MockProgressRepo, MockPreferencesRepo, MockCatalogRepo>;
type JobSvc = JobServiceImpl<MockJobStore>;
type DiscoverySvc = DiscoveryServiceImpl<
    MockCatalogRepo,
    MockSearchIndex,
    MockProgressRepo,
    MockPreferencesRepo,
    MockUserRepo,
>;
type LibrarySvc = LibraryServiceImpl<
    MockLibraryRepo,
    MockUserRepo,
    MockJobStore,
    MockCatalogRepo,
    MockMetadataProvider,
>;

struct Ctx {
    auth: MockAuthService,
    catalog_repo: MockCatalogRepo,
    users_repo: MockUserRepo,
    library_repo: MockLibraryRepo,
    jobs_repo: MockJobStore,
    catalog: CatalogSvc,
    session: MockSessionService,
    library: LibrarySvc,
    progress_repo: MockProgressRepo,
    preferences_repo: MockPreferencesRepo,
    search_index: MockSearchIndex,
    user: UserSvc,
    user_library: UserLibrarySvc,
    discovery: DiscoverySvc,
    job: JobSvc,
}

impl Ctx {
    fn new() -> Self {
        let auth = MockAuthService::new();
        auth.add_account("admin", "pw", UserId("admin".into()), Role::Admin);
        auth.add_account("user", "pw", UserId("u1".into()), Role::User);
        let catalog_repo = MockCatalogRepo::new();
        let users_repo = MockUserRepo::new();
        users_repo.insert(account("admin", Role::Admin));
        users_repo.insert(account("u1", Role::User));
        // Admin is no longer implicitly granted every library, so the fixture grants
        // it the libraries these tests use, the same way a real admin would be.
        users_repo.grant(
            &UserId("admin".into()),
            &[LibraryId("lib1".into()), LibraryId("ext1".into())],
        );
        let library_repo = MockLibraryRepo::new();
        let jobs_repo = MockJobStore::new();
        let progress_repo = MockProgressRepo::new();
        let preferences_repo = MockPreferencesRepo::new();
        let search_index = MockSearchIndex::new();
        Ctx {
            auth,
            catalog: CatalogServiceImpl::new(catalog_repo.clone(), users_repo.clone()),
            library: LibraryServiceImpl::new(
                library_repo.clone(),
                users_repo.clone(),
                jobs_repo.clone(),
                catalog_repo.clone(),
                None::<MockMetadataProvider>,
            )
            .with_enrichment_flags(true, true, true)
            .with_content_fetch(true),
            user: UserServiceImpl::new(
                users_repo.clone(),
                MockAuthTokenRepo::new(),
                MockUserDataStore::new(),
            ),
            user_library: UserLibraryServiceImpl::new(
                Arc::new(progress_repo.clone()),
                Arc::new(preferences_repo.clone()),
                Arc::new(catalog_repo.clone()),
            ),
            job: JobServiceImpl::new(jobs_repo.clone()),
            discovery: DiscoveryServiceImpl::new(
                catalog_repo.clone(),
                search_index.clone(),
                progress_repo.clone(),
                preferences_repo.clone(),
                users_repo.clone(),
            ),
            catalog_repo,
            users_repo,
            library_repo,
            jobs_repo,
            progress_repo,
            preferences_repo,
            search_index,
            session: MockSessionService::new(),
        }
    }

    fn grant(&self, user: &str, libraries: &[&str]) {
        let ids: Vec<LibraryId> = libraries
            .iter()
            .map(|lib| LibraryId((*lib).to_owned()))
            .collect();
        self.users_repo.grant(&UserId(user.to_owned()), &ids);
    }

    fn app(&self) -> Router {
        router(self.state())
    }

    fn webhook_app(&self, clients: Vec<WebhookClient>) -> Router {
        webhook_router(self.state(), clients)
    }

    fn state(
        &self,
    ) -> AppState<
        MockAuthService,
        CatalogSvc,
        MockSessionService,
        LibrarySvc,
        UserSvc,
        UserLibrarySvc,
        DiscoverySvc,
        JobSvc,
    > {
        AppState::new(
            self.auth.clone(),
            self.catalog.clone(),
            self.session.clone(),
            self.library.clone(),
            self.user.clone(),
            self.user_library.clone(),
            self.discovery.clone(),
            self.job.clone(),
        )
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

fn account(id: &str, role: Role) -> User {
    User {
        id: UserId(id.to_owned()),
        username: id.to_owned(),
        password_hash: "hash".into(),
        role,
        max_content_rating: None,
        preferred_audio: Vec::new(),
        preferred_subtitle: Vec::new(),
        concurrent_stream_limit: None,
        bitrate_cap: None,
        active: true,
        created_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
    }
}

fn movie(id: &str) -> Movie {
    Movie {
        id: MovieId(id.into()),
        title: format!("Alpha {id}"),
        sort_title: format!("alpha {id}"),
        year: Some(2020),
        overview: Some("overview".into()),
        runtime_minutes: Some(100),
        content_rating: Some(ContentRating {
            system: "MPAA".into(),
            code: "PG-13".into(),
        }),
        manually_edited: false,
        added_at: Timestamp::now(),
        updated_at: Timestamp::now(),
        artwork: Vec::new(),
    }
}

fn series(id: &str) -> Series {
    Series {
        id: SeriesId(id.into()),
        title: format!("Alpha {id}"),
        sort_title: format!("alpha {id}"),
        year: Some(2019),
        overview: None,
        content_rating: Some(ContentRating {
            system: "TV".into(),
            code: "TV-14".into(),
        }),
        manually_edited: false,
        added_at: Timestamp::now(),
        updated_at: Timestamp::now(),
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
        added_at: Timestamp::now(),
        updated_at: Timestamp::now(),
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
        manually_edited: false,
        added_at: Timestamp::now(),
        updated_at: Timestamp::now(),
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
        available: true,
        added_at: Timestamp::now(),
        updated_at: Timestamp::now(),
    }
}

fn queued_job(id: &str) -> domain::job::Job {
    use domain::job::{Job, JobId, JobKind, JobPriority, JobStatus};
    Job {
        id: JobId(id.into()),
        kind: JobKind::LibraryScan,
        status: JobStatus::Queued,
        priority: JobPriority::Normal,
        payload: "lib1".into(),
        attempts: 0,
        progress: 0.0,
        available_at: Timestamp::UNIX_EPOCH,
        last_error: None,
        created_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
        started_at: None,
        finished_at: None,
        parent_id: None,
    }
}

fn external_library(id: &str) -> Library {
    Library {
        origin: LibraryOrigin::External,
        ..library(id)
    }
}

fn audio_track(index: u32) -> AudioTrack {
    AudioTrack {
        index,
        codec: "aac".into(),
        channels: 2,
        language: Some(LanguageCode("eng".into())),
        bitrate: None,
    }
}

fn subtitle_file(id: &str) -> SubtitleFile {
    SubtitleFile {
        id: SubtitleFileId(id.into()),
        version: VersionId("v1".into()),
        language: Some(LanguageCode("eng".into())),
        format: SubtitleFormat::Srt,
        source: SubtitleSource::External,
        path: format!("/media/{id}.srt"),
        translated_from: None,
        label: None,
        pinned: false,
    }
}

fn library(id: &str) -> Library {
    Library {
        id: LibraryId(id.into()),
        name: format!("Lib {id}"),
        origin: LibraryOrigin::Local,
        kind: LibraryKind::Movie,
        roots: vec!["/media".into()],
        sort_articles: vec!["the".into()],
        watcher: WatcherStrategy::Manual,
        scan_schedule: Some("0 0 * * *".into()),
        metadata_sources: vec!["tmdb".into()],
        created_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
    }
}

#[tokio::test]
async fn auth_endpoints() {
    let ctx = Ctx::new();
    ctx.auth.add_link_code(
        "7G2K9QMP",
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
        Some(json!({"code": "7G2K9QMP", "device": {"name": "Roku", "platform": "roku"}})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["token"], "player-token");
}

// Only /admin/versions had a delete test, so the success path of the four title
// delete endpoints was never taken. These are the destructive routes, and a 204
// that never ran is a 204 nobody has seen.
#[tokio::test]
async fn the_admin_title_deletes_are_gated_and_return_no_content() {
    let ctx = Ctx::new();
    ctx.catalog_repo.add_movie(movie("m1"));
    ctx.catalog_repo.add_series(series("s1"));
    ctx.catalog_repo.add_season(season("se1", "s1"));
    ctx.catalog_repo.add_episode(episode("e1", "se1"));

    // Leaves first: a title with children still under it is refused.
    let uris = [
        "/api/v1/admin/episodes/e1",
        "/api/v1/admin/seasons/se1",
        "/api/v1/admin/series/s1",
        "/api/v1/admin/movies/m1",
    ];
    for uri in uris {
        let (status, _) = call(ctx.app(), Method::DELETE, uri, Some(USER), None).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "DELETE {uri} as user");
        let (status, _) = call(ctx.app(), Method::DELETE, uri, Some(ADMIN), None).await;
        assert_eq!(status, StatusCode::NO_CONTENT, "DELETE {uri} as admin");
    }

    let (status, _) = call(
        ctx.app(),
        Method::DELETE,
        "/api/v1/admin/movies/m1",
        Some(ADMIN),
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "deleting the same title twice is not found, not another 204"
    );
}

#[tokio::test]
async fn self_reports_the_session_role_not_the_account_role() {
    let ctx = Ctx::new();
    ctx.users_repo.insert(account("boss", Role::Admin));
    ctx.auth
        .add_account("tablet", "pw", UserId("boss".into()), Role::Player);

    let (status, body) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/users/self",
        Some("access:boss"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], "boss");
    assert_eq!(
        body["role"], "player",
        "a linked device told it is an admin renders admin pages the server then refuses"
    );

    let (status, body) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/users/boss",
        Some(ADMIN),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["role"], "admin",
        "the account row is still the account row; only self is session scoped"
    );
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

    // A non-admin may create a link code only for their own account.
    let (status, _) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/auth/link/create",
        Some(USER),
        Some(json!({"user_id": "admin"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/auth/link/create",
        Some(USER),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn catalog_routes() {
    let ctx = Ctx::new();
    ctx.catalog_repo.add_movie(movie("m1"));
    ctx.catalog_repo.add_series(series("s1"));
    ctx.catalog_repo.add_season(season("se1", "s1"));
    ctx.catalog_repo.add_episode(episode("e1", "se1"));
    ctx.catalog_repo.add_collection(Collection {
        id: CollectionId("c1".into()),
        name: "Saga".into(),
        overview: Some("epic".into()),
        movies: vec![MovieId("m1".into())],
        added_at: Timestamp::now(),
        updated_at: Timestamp::now(),
        artwork: Vec::new(),
    });
    for (vid, q) in [
        ("v1", Quality::Sd),
        ("v2", Quality::Hd),
        ("v3", Quality::Fhd),
        ("v4", Quality::Uhd),
    ] {
        ctx.catalog_repo.add_version(version(
            vid,
            TitleId::Movie(MovieId("m1".into())),
            "lib1",
            q,
        ));
    }
    ctx.catalog_repo.add_version(version(
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
    ctx.grant("u1", &["lib1"]);

    for (uri, expected) in cases {
        let (status, _) = call(ctx.app(), Method::GET, uri, Some(USER), None).await;
        assert_eq!(status, expected, "GET {uri}");
    }

    // The list routes are now served by the real service, so prove they carry rows rather
    // than an empty page that would satisfy the status assertions above just as well.
    for uri in ["/api/v1/movies", "/api/v1/series"] {
        let (_, body) = call(ctx.app(), Method::GET, uri, Some(USER), None).await;
        assert!(
            !body["items"].as_array().unwrap().is_empty(),
            "GET {uri} returned an empty page"
        );
    }

    // Batch episode cards carry resolved show context (series id/title + season number).
    let (status, body) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/titles/batch",
        Some(USER),
        Some(json!({"titles": [{"type": "episode", "id": "e1"}]})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["type"], "episode");
    assert_eq!(body[0]["series_id"], "s1");
    assert_eq!(body[0]["series_title"], "Alpha s1");
    assert_eq!(body[0]["season_number"], 1);

    // People cards come back in one batch, and an oversized request is refused.
    ctx.catalog_repo.add_person(Person {
        id: PersonId("p1".into()),
        name: "Ada Lovelace".into(),
        ..Person::default()
    });
    let (status, body) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/people/batch",
        Some(USER),
        Some(json!({"people": ["p1", "ghost"]})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body.as_array().map(Vec::len),
        Some(1),
        "a person the catalog does not know is skipped, not an error"
    );
    let too_many_people: Vec<Value> = (0..201).map(|i| json!(format!("p{i}"))).collect();
    let (status, _) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/people/batch",
        Some(USER),
        Some(json!({"people": too_many_people})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Collection detail embeds resolved member movie cards alongside the id list.
    let (status, body) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/movies/collections/c1",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["movies"][0], "m1");
    assert_eq!(body["items"][0]["id"], "m1");

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
async fn the_catalog_routes_enforce_library_access_and_the_rating_cap() {
    let ctx = Ctx::new();
    ctx.catalog_repo.add_movie(movie("m1"));
    ctx.catalog_repo.add_version(version(
        "v1",
        TitleId::Movie(MovieId("m1".into())),
        "lib1",
        Quality::Hd,
    ));

    // u1 holds no grant at all, so the library it lives in is not theirs to see.
    let (status, body) = call(ctx.app(), Method::GET, "/api/v1/movies", Some(USER), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body["items"].as_array().unwrap().is_empty(),
        "a user with no library grant must not see the catalog"
    );

    // An admin is not scoped by grants, so the same request carries the row.
    let (_, body) = call(ctx.app(), Method::GET, "/api/v1/movies", Some(ADMIN), None).await;
    assert_eq!(body["items"][0]["id"], "m1");

    // Granting the library is what makes it visible, and the versions follow.
    ctx.grant("u1", &["lib1"]);
    let (_, body) = call(ctx.app(), Method::GET, "/api/v1/movies", Some(USER), None).await;
    assert_eq!(body["items"][0]["id"], "m1");
    let (_, body) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/movies/m1/versions",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(body["items"][0]["id"], "v1");

    // A cap below the title's rating hides it again, grant or no grant. The fixture movie is
    // PG-13, so a PG-13 cap keeps m1 and must reject the R-rated m2 added below.
    ctx.users_repo.insert(User {
        max_content_rating: Some(ContentRating {
            system: "MPAA".into(),
            code: "PG-13".into(),
        }),
        ..account("u1", Role::User)
    });
    let rated = Movie {
        content_rating: Some(ContentRating {
            system: "MPAA".into(),
            code: "R".into(),
        }),
        ..movie("m2")
    };
    ctx.catalog_repo.add_movie(rated);
    ctx.catalog_repo.add_version(version(
        "v2",
        TitleId::Movie(MovieId("m2".into())),
        "lib1",
        Quality::Hd,
    ));

    let (_, body) = call(ctx.app(), Method::GET, "/api/v1/movies", Some(USER), None).await;
    let ids: Vec<&str> = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        ["m1"],
        "the R-rated title must not reach a G-capped user"
    );

    let (status, _) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/movies/m2",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "the detail route must gate on the cap too"
    );
}

#[tokio::test]
async fn remote_content_cannot_be_fetched_into_a_local_library() {
    let ctx = Ctx::new();
    ctx.library_repo.insert_library(library("lib1"));

    let (status, _) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/admin/fetch",
        Some(ADMIN),
        Some(json!({
            "source_url": "https://x/v",
            "kind": "movie",
            "library_id": "lib1",
            "title": "The Matrix"
        })),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "fetched content only belongs in a library that was declared external"
    );
}

#[tokio::test]
async fn saving_a_title_puts_the_watchlist_row_at_the_top_of_the_hub() {
    let ctx = Ctx::new();
    ctx.grant("u1", &["lib1"]);
    ctx.catalog_repo.add_movie(movie("m1"));
    ctx.catalog_repo.add_version(version(
        "v1",
        TitleId::Movie(MovieId("m1".into())),
        "lib1",
        Quality::Hd,
    ));
    ctx.preferences_repo.seed_watchlist(WatchlistItem {
        user: UserId("u1".into()),
        title: TitleId::Movie(MovieId("m1".into())),
        added_at: Timestamp::UNIX_EPOCH,
    });

    let (status, body) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/users/u1/hub",
        Some(USER),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let hubs = body.as_array().unwrap();
    assert_eq!(
        hubs[0]["id"], "watchlist",
        "a saved title has to lead the hub, not sit below the generated rows"
    );
    assert_eq!(hubs[0]["items"][0]["id"], "m1");
}

#[tokio::test]
async fn an_accepted_scan_actually_reaches_the_job_queue() {
    use domain::job::JobKind;
    use domain::repository::JobRepository;

    let ctx = Ctx::new();
    ctx.library_repo.insert_library(library("lib1"));

    let (status, _) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/libraries/lib1/scan",
        Some(ADMIN),
        Some(json!({})),
    )
    .await;

    assert_eq!(status, StatusCode::ACCEPTED);
    let queued = ctx.jobs_repo.list().await.unwrap();
    assert!(
        queued.iter().any(|job| job.kind == JobKind::LibraryScan),
        "202 must mean the work was queued, not merely that the route answered"
    );
}

#[tokio::test]
async fn a_library_is_invisible_to_a_user_who_was_never_granted_it() {
    let ctx = Ctx::new();
    ctx.library_repo.insert_library(library("lib1"));

    let (status, _) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/libraries/lib1",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "an ungranted library must not even confirm it exists"
    );

    let (_, body) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/libraries",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(
        body.as_array().map(Vec::len),
        Some(0),
        "and it must not appear in the list either"
    );
}

#[tokio::test]
async fn library_routes() {
    let ctx = Ctx::new();
    let id = LibraryId("lib1".into());
    ctx.library_repo.insert_library(library("lib1"));
    ctx.grant("u1", &["lib1"]);
    ctx.library_repo.add_unmatched(UnmatchedFile {
        id: UnmatchedFileId("uf1".into()),
        library: id.clone(),
        path: "/media/x.mkv".into(),
        candidates: vec![MatchCandidate {
            title: TitleId::Movie(MovieId("m1".into())),
            confidence: 0.9,
            label: "Alpha".into(),
        }],
        created_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
    });
    ctx.library_repo.add_duplicate(
        &id,
        DuplicateCandidate {
            id: DuplicateCandidateId("d1".into()),
            title: TitleId::Episode(EpisodeId("e1".into())),
            paths: vec!["/a.mkv".into(), "/b.mkv".into()],
        },
    );
    ctx.catalog_repo.add_version(version(
        "v1",
        TitleId::Movie(MovieId("m1".into())),
        "lib1",
        Quality::Hd,
    ));
    // The real service validates that the track and subtitles a trigger names
    // actually exist on the version, so they have to be seeded.
    ctx.catalog_repo
        .set_version_tracks(&VersionId("v1".into()), &[], &[audio_track(2)], &[], &[])
        .await
        .unwrap();
    ctx.catalog_repo
        .set_subtitle_files(
            &VersionId("v1".into()),
            &[
                subtitle_file("sf1"),
                subtitle_file("sf-en"),
                subtitle_file("sf-fr"),
            ],
        )
        .await
        .unwrap();

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
        "/api/v1/admin/versions",
    ];
    for uri in admin_gets {
        let (status, _) = call(ctx.app(), Method::GET, uri, Some(ADMIN), None).await;
        assert_eq!(status, StatusCode::OK, "GET {uri}");
    }
    // The global versions table is admin-only.
    let (status, _) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/admin/versions",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let triggers = [
        (
            "/api/v1/admin/versions/v1/transcribe",
            json!({"audio_track_index": 2}),
        ),
        (
            "/api/v1/admin/versions/v1/translate",
            json!({"source_subtitle_id": "sf1", "target_language": "zh"}),
        ),
        (
            "/api/v1/admin/versions/v1/upscale",
            json!({"target_height": 2160}),
        ),
        (
            "/api/v1/admin/versions/v1/subtitles/combine",
            json!({"top_subtitle_id": "sf-en", "bottom_subtitle_id": "sf-fr"}),
        ),
        ("/api/v1/people/p1/refresh", json!({})),
    ];
    for (uri, body) in triggers {
        let (status, _) = call(ctx.app(), Method::POST, uri, Some(USER), Some(body.clone())).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "POST {uri} as user");
        let (status, _) = call(ctx.app(), Method::POST, uri, Some(ADMIN), Some(body)).await;
        assert_eq!(status, StatusCode::ACCEPTED, "POST {uri} as admin");
    }
    let delete_uri = "/api/v1/admin/versions/v1";
    let (status, _) = call(ctx.app(), Method::DELETE, delete_uri, Some(USER), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "DELETE {delete_uri} as user");
    let (status, _) = call(ctx.app(), Method::DELETE, delete_uri, Some(ADMIN), None).await;
    assert_eq!(
        status,
        StatusCode::NO_CONTENT,
        "DELETE {delete_uri} as admin"
    );
    ctx.jobs_repo.seed(queued_job("j1"));
    let cancel_uri = "/api/v1/admin/jobs/j1/cancel";
    let (status, _) = call(ctx.app(), Method::POST, cancel_uri, Some(USER), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "POST {cancel_uri} as user");
    let (status, _) = call(ctx.app(), Method::POST, cancel_uri, Some(ADMIN), None).await;
    assert_eq!(status, StatusCode::ACCEPTED, "POST {cancel_uri} as admin");
    let (status, _) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/admin/versions/v1/upscale",
        Some(ADMIN),
        Some(json!({"target_height": 0})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/admin/versions/v1/subtitles/combine",
        Some(ADMIN),
        Some(json!({"top_subtitle_id": "sf-en", "bottom_subtitle_id": "sf-en"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    // Content fetch is admin-only and validates tv season/episode.
    ctx.library_repo.insert_library(external_library("ext1"));
    let fetch_body = json!({"source_url": "https://x/v", "kind": "movie", "library_id": "ext1", "title": "The Matrix"});
    let (status, _) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/admin/fetch",
        Some(USER),
        Some(fetch_body.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/admin/fetch",
        Some(ADMIN),
        Some(fetch_body),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let (status, _) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/admin/fetch",
        Some(ADMIN),
        Some(json!({"source_url": "https://x/v", "kind": "tv", "library_id": "lib1", "title": "Show"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
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

    // Trigger a scan, then confirm the state flips to queued.
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
    assert_eq!(body["status"], "queued");

    // The library metadata refresh is admin-only and does not scan.
    let refresh_uri = "/api/v1/libraries/lib1/refresh-metadata";
    let (status, _) = call(ctx.app(), Method::POST, refresh_uri, Some(USER), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = call(ctx.app(), Method::POST, refresh_uri, Some(ADMIN), None).await;
    assert_eq!(status, StatusCode::ACCEPTED);
}

#[tokio::test]
async fn webhook_routes() {
    let clients = || {
        vec![WebhookClient {
            name: "sonarr".into(),
            secret: "sec".into(),
            libraries: vec!["lib1".into(), "missing".into()],
        }]
    };

    let ctx = Ctx::new();
    ctx.library_repo.insert_library(library("lib1"));
    let (status, _) = call(
        ctx.webhook_app(clients()),
        Method::POST,
        "/api/v1/webhooks/libraries/lib1/scan?token=sec",
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let (_, body) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/libraries/lib1/scan",
        Some(ADMIN),
        None,
    )
    .await;
    assert_eq!(body["status"], "queued");

    let ctx = Ctx::new();
    ctx.library_repo.insert_library(library("lib1"));
    let (status, _) = call(
        ctx.webhook_app(clients()),
        Method::POST,
        "/api/v1/webhooks/libraries/lib1/scan",
        Some("sec"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);

    let ctx = Ctx::new();
    ctx.library_repo.insert_library(library("lib1"));
    for uri in [
        "/api/v1/webhooks/libraries/lib1/scan?token=wrong",
        "/api/v1/webhooks/libraries/lib1/scan",
    ] {
        let (status, _) = call(ctx.webhook_app(clients()), Method::POST, uri, None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "POST {uri}");
    }

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/webhooks/libraries/lib1/scan")
        .header(header::AUTHORIZATION, "Basic Zm9v")
        .body(Body::empty())
        .unwrap();
    let response = ctx.webhook_app(clients()).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let (status, _) = call(
        ctx.webhook_app(clients()),
        Method::POST,
        "/api/v1/webhooks/libraries/lib2/scan?token=sec",
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _) = call(
        ctx.webhook_app(clients()),
        Method::POST,
        "/api/v1/webhooks/libraries/missing/scan?token=sec",
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = call(
        ctx.webhook_app(Vec::new()),
        Method::POST,
        "/api/v1/webhooks/libraries/lib1/scan?token=sec",
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let ctx = Ctx::new();
    ctx.library_repo.insert_library(library("lib1"));
    for _ in 0..2 {
        let (status, _) = call(
            ctx.webhook_app(clients()),
            Method::POST,
            "/api/v1/webhooks/libraries/lib1/scan?token=sec",
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::ACCEPTED);
    }

    let ctx = Ctx::new();
    ctx.library_repo.insert_library(library("lib1"));
    let (status, _) = call(
        ctx.webhook_app(clients()),
        Method::POST,
        "/api/v1/webhooks/libraries/lib1/scan?token=sec",
        None,
        Some(json!({"eventType": "Test"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, body) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/libraries/lib1/scan",
        Some(ADMIN),
        None,
    )
    .await;
    assert_eq!(body["status"], "idle");

    let (status, _) = call(
        ctx.webhook_app(clients()),
        Method::POST,
        "/api/v1/webhooks/libraries/lib1/scan?token=sec",
        None,
        Some(json!({"eventType": "Download"})),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let (_, body) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/libraries/lib1/scan",
        Some(ADMIN),
        None,
    )
    .await;
    assert_eq!(body["status"], "queued");
}

#[tokio::test]
async fn session_routes() {
    let ctx = Ctx::new();
    // now_playing resolves each session against the catalog, so a session whose
    // version is not there is correctly dropped from the activity list.
    ctx.catalog_repo.add_movie(movie("m1"));
    ctx.catalog_repo.add_version(version(
        "v1",
        TitleId::Movie(MovieId("m1".into())),
        "lib1",
        Quality::Hd,
    ));

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

    let (status, _) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/users",
        Some(ADMIN),
        Some(json!({"username": "robot", "password": "pw", "role": "automation"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    for blank in ["", "   "] {
        let (status, body) = call(
            ctx.app(),
            Method::POST,
            "/api/v1/users",
            Some(ADMIN),
            Some(json!({"username": "blank", "password": blank, "role": "user"})),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"]["code"], "empty_password");
    }

    for blank in ["", "   "] {
        let (status, body) = call(
            ctx.app(),
            Method::PUT,
            &format!("/api/v1/users/{uid}/password"),
            Some(ADMIN),
            Some(json!({"new_password": blank})),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"]["code"], "empty_password");
    }

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

    // Self access, the Ok branch of admin-or-self.
    let (status, body) = call(ctx.app(), Method::GET, "/api/v1/users/u1", Some(USER), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], "u1", "a user may read their own account");

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

    let (status, body) = call(
        ctx.app(),
        Method::DELETE,
        "/api/v1/users/admin",
        Some(ADMIN),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"], "cannot_delete_self");

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
async fn set_active_toggles_a_user_and_refuses_self_deactivation() {
    let ctx = Ctx::new();
    let (status, created) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/users",
        Some(ADMIN),
        Some(json!({"username": "switchable", "password": "pw", "role": "user"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let uid = created["id"].as_str().unwrap().to_owned();
    assert_eq!(created["active"], true);

    let (status, body) = call(
        ctx.app(),
        Method::PUT,
        &format!("/api/v1/users/{uid}/active"),
        Some(ADMIN),
        Some(json!({"active": false})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["active"], false);

    let (status, body) = call(
        ctx.app(),
        Method::GET,
        &format!("/api/v1/users/{uid}"),
        Some(ADMIN),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["active"], false);

    let (status, body) = call(
        ctx.app(),
        Method::PUT,
        "/api/v1/users/admin/active",
        Some(ADMIN),
        Some(json!({"active": false})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"], "cannot_deactivate_self");

    let (status, _) = call(
        ctx.app(),
        Method::PUT,
        &format!("/api/v1/users/{uid}/active"),
        Some(USER),
        Some(json!({"active": true})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn deactivating_and_deleting_both_stop_playback_in_flight() {
    for deactivate in [true, false] {
        let ctx = Ctx::new();
        ctx.users_repo.insert(account("u1", Role::User));
        let (status, session) = call(
            ctx.app(),
            Method::POST,
            "/api/v1/sessions",
            Some(USER),
            Some(json!({
                "version_id": "v1",
                "capabilities": {
                    "platform": "web",
                    "profile_version": 1,
                    "max_bitrate": null
                }
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "could not start playback");
        let sid = session["session_id"].as_str().unwrap().to_owned();

        if deactivate {
            let (status, _) = call(
                ctx.app(),
                Method::PUT,
                "/api/v1/users/u1/active",
                Some(ADMIN),
                Some(json!({"active": false})),
            )
            .await;
            assert_eq!(status, StatusCode::OK);
        } else {
            let (status, _) = call(
                ctx.app(),
                Method::DELETE,
                "/api/v1/users/u1",
                Some(ADMIN),
                None,
            )
            .await;
            assert_eq!(status, StatusCode::NO_CONTENT);
        }

        let (status, body) = call(
            ctx.app(),
            Method::GET,
            "/api/v1/users/activity",
            Some(ADMIN),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            body["items"].as_array().unwrap().is_empty(),
            "the playback session [{sid}] outlived the account"
        );
    }
}

#[tokio::test]
async fn user_library_routes() {
    let ctx = Ctx::new();
    let u1 = UserId("u1".into());
    ctx.progress_repo.seed_history(WatchHistory {
        user: u1.clone(),
        title: TitleId::Movie(MovieId("m1".into())),
        watched: true,
        play_count: 2,
        last_watched_at: Some(Timestamp::now()),
        completed: false,
    });
    ctx.progress_repo.seed_progress(PlaybackProgress {
        user: u1.clone(),
        version: VersionId("v1".into()),
        position_ms: 1234,
        audio_track: None,
        subtitle: None,
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

    let (status, _) = call(
        ctx.app(),
        Method::DELETE,
        "/api/v1/users/u1/history/m1",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, body) = call(
        ctx.app(),
        Method::GET,
        "/api/v1/users/u1/history",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["items"].as_array().unwrap().is_empty());

    let (status, _) = call(
        ctx.app(),
        Method::DELETE,
        "/api/v1/users/u1/history",
        Some(USER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

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

    // Watched rollup for containers (season/series only).
    let (status, body) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/users/u1/state/rollup",
        Some(USER),
        Some(json!({"targets": [{"type": "series", "id": "s1"}, {"type": "season", "id": "se1"}]})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 2);
    assert_eq!(body[0]["target"], json!({"type": "series", "id": "s1"}));
    assert_eq!(body[0]["total_episodes"], 0);

    // Movie/episode targets are rejected; those use state/batch instead.
    let (status, _) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/users/u1/state/rollup",
        Some(USER),
        Some(json!({"targets": [{"type": "movie", "id": "m1"}]})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Over the batch cap is rejected.
    let too_many: Vec<Value> = (0..201)
        .map(|i| json!({"type": "season", "id": format!("se{i}")}))
        .collect();
    let (status, _) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/users/u1/state/rollup",
        Some(USER),
        Some(json!({"targets": too_many})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn discovery_routes() {
    let ctx = Ctx::new();
    ctx.grant("u1", &["lib1"]);
    ctx.search_index.add(SearchResult::Movie(movie("m1")));
    ctx.search_index.add(SearchResult::Series(series("s1")));
    ctx.search_index
        .add(SearchResult::Episode(episode("e1", "se1")));
    ctx.search_index.add(SearchResult::Person(Person {
        id: PersonId("p1".into()),
        name: "Alpha Person".into(),
        ..Person::default()
    }));
    // The real service derives every rail from the catalog, so the catalog is
    // what has to be seeded rather than the rails themselves.
    ctx.catalog_repo.add_movie(movie("m1"));
    ctx.catalog_repo.add_movie(movie("m2"));
    ctx.catalog_repo.add_series(series("s1"));
    ctx.catalog_repo.add_season(season("se1", "s1"));
    ctx.catalog_repo.add_episode(episode("e1", "se1"));
    ctx.catalog_repo.add_episode(Episode {
        number: 2,
        ..episode("e2", "se1")
    });
    ctx.catalog_repo.add_version(version(
        "v1",
        TitleId::Movie(MovieId("m1".into())),
        "lib1",
        Quality::Hd,
    ));
    // A title is only visible through a version in a granted library, so every
    // title a rail can name needs one: the watched episode, the episode that
    // follows it, and the follow-up movie.
    ctx.catalog_repo.add_version(version(
        "ev1",
        TitleId::Episode(EpisodeId("e1".into())),
        "lib1",
        Quality::Hd,
    ));
    ctx.catalog_repo.add_version(version(
        "ev2",
        TitleId::Episode(EpisodeId("e2".into())),
        "lib1",
        Quality::Hd,
    ));
    ctx.catalog_repo.add_version(version(
        "v2",
        TitleId::Movie(MovieId("m2".into())),
        "lib1",
        Quality::Hd,
    ));
    // Both next-up rails are earned, never declared: each needs a watched
    // predecessor before the service will suggest what follows it.
    ctx.catalog_repo.add_collection(Collection {
        id: CollectionId("c1".into()),
        name: "Alpha Collection".into(),
        overview: None,
        movies: vec![MovieId("m1".into()), MovieId("m2".into())],
        artwork: Vec::new(),
        added_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
    });
    for title in [
        TitleId::Movie(MovieId("m1".into())),
        TitleId::Episode(EpisodeId("e1".into())),
    ] {
        ctx.progress_repo.seed_history(WatchHistory {
            user: UserId("u1".into()),
            title,
            watched: true,
            play_count: 1,
            last_watched_at: Some(Timestamp::UNIX_EPOCH),
            completed: true,
        });
    }
    // Past the 5% start threshold and short of the 90% completion mark, so the
    // version counts as genuinely in progress.
    ctx.progress_repo.seed_progress(PlaybackProgress {
        user: UserId("u1".into()),
        version: VersionId("v1".into()),
        position_ms: 200,
        audio_track: None,
        subtitle: None,
        updated_at: Timestamp::now(),
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

    // A session for v1, which already has a stored progress row. The two must not
    // both surface, or the client rail shows the same title twice.
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
    assert!(body["now_playing"].as_array().unwrap().is_empty());
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
    let hubs = body.as_array().unwrap();
    let ids: Vec<&str> = hubs.iter().map(|h| h["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids,
        [
            "recently_added_movies",
            "recently_added_shows",
            "on_deck",
            "continue_watching"
        ],
        "every row is derived, and the empty watchlist row is dropped rather than sent"
    );
    let show = &hubs[1]["items"][0];
    assert_eq!(show["type"], "series");
    assert_eq!(show["id"], "s1");
    assert_eq!(show["episode_count"], 2);
}

#[tokio::test]
async fn a_session_that_never_left_the_start_stays_off_the_continue_rail() {
    let ctx = Ctx::new();
    ctx.grant("u1", &["lib1"]);
    ctx.catalog_repo.add_movie(movie("m1"));
    ctx.catalog_repo.add_version(version(
        "v1",
        TitleId::Movie(MovieId("m1".into())),
        "lib1",
        Quality::Hd,
    ));

    let (status, _) = call(
        ctx.app(),
        Method::POST,
        "/api/v1/sessions",
        Some(USER),
        Some(json!({
            "version_id": "v1",
            "capabilities": {"platform": "web", "profile_version": 1, "max_bitrate": null},
            "audio_track": null,
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
    // The session is real but has no position to offer, and the title page would
    // show Play rather than Resume, so a card here would contradict it.
    assert!(body["now_playing"].as_array().unwrap().is_empty());
    assert!(body["in_progress"].as_array().unwrap().is_empty());
}

use std::path::Path;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use jiff::Timestamp;
use serde_json::{Value, json};
use tower::ServiceExt;

use contracts::{Token, fixture, requests};
use domain::catalog::{
    ArtworkOwner, CollectionId, EpisodeId, MovieId, SeasonId, SeriesId, TitleId, TitleRef, Version,
    VersionDetail, VersionId,
};
use domain::common::{LanguageCode, Quality};
use domain::library::{LibraryId, MatchCandidate, UnmatchedFile, UnmatchedFileId};
use domain::media::{
    AudioTrack, DetectedMarkers, EmbeddedSubtitleTrack, SubtitleFile, SubtitleFileId,
    SubtitleFormat, SubtitleSource, VideoTrack,
};
use domain::playback::PlaybackProgress;
use domain::repository::{
    AuthTokenRepository, CatalogRepository, JobRepository, LibraryRepository, ProgressRepository,
    SearchIndex, UserRepository,
};
use domain::user::{PendingLink, Role, User, UserId};
use server::{
    Built, CancelRegistry, CapabilityInputs, Repos, WireConfig, app, build_state,
    server_capabilities,
};
use services::password;

const LINK_EXPIRES_AT: i64 = 4_102_444_800;

fn config(root: &Path) -> WireConfig {
    WireConfig {
        jwt_secret: b"replay-jwt-secret-key".to_vec(),
        stream_secret: b"replay-stream-secret-key".to_vec(),
        access_ttl_secs: 3600,
        refresh_ttl_secs: 86_400,
        transcode_cache: root.join("transcode"),
        artwork_cache: root.join("artwork"),
        trickplay_cache: root.join("trickplay"),
        tmdb_api_key: None,
        transcription_enabled: true,
        translation_enabled: true,
        upscaling_enabled: true,
        content_fetch_enabled: false,
        fetch_cookies_file: None,
        vaapi_device: None,
        remux_read_rate: 10.0,
        max_transcode_height: None,
        profile_overrides_dir: None,
    }
}

fn user(id: &str, username: &str, role: Role, hash: &str) -> User {
    User {
        id: UserId(id.into()),
        username: username.into(),
        password_hash: hash.to_owned(),
        role,
        max_content_rating: None,
        preferred_audio: Vec::new(),
        preferred_subtitle: Vec::new(),
        concurrent_stream_limit: None,
        bitrate_cap: None,
        active: true,
        created_at: fixture::ts(0),
        updated_at: fixture::ts(0),
    }
}

fn direct_v1_detail() -> VersionDetail {
    VersionDetail {
        version: Version {
            id: VersionId("v1".into()),
            title: TitleId::Movie(MovieId("m1".into())),
            library: LibraryId("lib1".into()),
            quality: Quality::Sd,
            container: "mp4".into(),
            path: "/media/v1.mp4".into(),
            size_bytes: 1,
            duration_ms: 100_000,
            available: true,
            added_at: fixture::ts(0),
            updated_at: fixture::ts(0),
        },
        video: vec![VideoTrack {
            index: 0,
            codec: "h264".into(),
            width: 1920,
            height: 1080,
            bit_depth: 8,
            hdr: None,
            frame_rate: 24.0,
            bitrate: Some(4_000_000),
        }],
        audio: vec![AudioTrack {
            index: 1,
            codec: "aac".into(),
            channels: 2,
            language: Some(LanguageCode("en".into())),
            bitrate: Some(128_000),
        }],
        subtitles: vec![EmbeddedSubtitleTrack {
            index: 1,
            language: Some(LanguageCode("en".into())),
            format: SubtitleFormat::Srt,
            forced: false,
            default: true,
        }],
        subtitle_files: vec![SubtitleFile {
            id: SubtitleFileId("v1-en".into()),
            version: VersionId("v1".into()),
            language: Some(LanguageCode("en".into())),
            format: SubtitleFormat::Srt,
            source: SubtitleSource::External,
            path: "/media/v1.en.srt".into(),
            translated_from: None,
            label: None,
            pinned: false,
        }],
        chapters: Vec::new(),
        markers: DetectedMarkers { intros: Vec::new(), credits: Vec::new() },
        trickplay: Vec::new(),
    }
}

async fn seed(repos: &Repos, hash: &str) {
    repos.catalog.insert_movie(fixture::movie("m1")).await.unwrap();
    repos.catalog.insert_series(fixture::series("s1")).await.unwrap();
    repos.catalog.insert_season(fixture::season("se1", "s1")).await.unwrap();
    repos.catalog.insert_episode(fixture::episode("e1", "se1")).await.unwrap();
    repos.catalog.insert_collection(fixture::saga_collection()).await.unwrap();

    for (owner, id) in [
        (ArtworkOwner::Movie(MovieId("m1".into())), "m1"),
        (ArtworkOwner::Series(SeriesId("s1".into())), "s1"),
        (ArtworkOwner::Season(SeasonId("se1".into())), "se1"),
        (ArtworkOwner::Episode(EpisodeId("e1".into())), "e1"),
        (ArtworkOwner::Collection(CollectionId("c1".into())), "c1"),
        (ArtworkOwner::Person(domain::metadata::PersonId("p1".into())), "p1"),
    ] {
        repos.catalog.set_artwork(&owner, &fixture::artwork_set(id)).await.unwrap();
    }

    repos.catalog.insert_version_detail(direct_v1_detail()).await.unwrap();
    for version in fixture::catalog_versions().into_iter().filter(|v| v.id.0 != "v1") {
        repos.catalog.insert_version(version).await.unwrap();
    }

    for person in fixture::people() {
        repos.catalog.upsert_person(person).await.unwrap();
    }
    repos
        .catalog
        .set_title_enrichment(&TitleRef::Movie(MovieId("m1".into())), &fixture::movie_enrichment())
        .await
        .unwrap();
    repos
        .catalog
        .set_title_enrichment(
            &TitleRef::Series(SeriesId("s1".into())),
            &fixture::series_enrichment(),
        )
        .await
        .unwrap();
    repos.catalog.rebuild().await.unwrap();

    repos.library.insert_library(fixture::library("lib1")).await.unwrap();
    repos
        .library
        .insert_unmatched(UnmatchedFile {
            id: UnmatchedFileId("uf1".into()),
            library: LibraryId("lib1".into()),
            path: "/media/unmatched.mkv".into(),
            candidates: vec![MatchCandidate {
                title: TitleId::Movie(MovieId("m1".into())),
                confidence: 0.9,
                label: "Alpha".into(),
            }],
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        })
        .await
        .unwrap();

    repos.users.create(user("admin", "admin", Role::Admin, hash)).await.unwrap();
    repos.users.create(user("u1", "user", Role::User, hash)).await.unwrap();
    repos
        .users
        .set_library_access(&UserId("u1".into()), &[LibraryId("lib1".into())])
        .await
        .unwrap();
    // Admin is no longer implicitly granted every library, so it is granted here
    // the same way a real admin would be.
    repos
        .users
        .set_library_access(&UserId("admin".into()), &[LibraryId("lib1".into())])
        .await
        .unwrap();

    repos
        .progress
        .upsert(PlaybackProgress {
            user: UserId("u1".into()),
            version: VersionId("v1".into()),
            position_ms: 1234,
            audio_track: None,
            subtitle: None,
            updated_at: fixture::ts(20),
        })
        .await
        .unwrap();

    repos
        .progress
        .upsert(PlaybackProgress {
            user: UserId("u1".into()),
            version: VersionId("ev1".into()),
            position_ms: 400,
            audio_track: None,
            subtitle: None,
            updated_at: fixture::ts(21),
        })
        .await
        .unwrap();

    repos
        .auth_tokens
        .store_link_code(PendingLink {
            code: fixture::LINK_CODE.into(),
            user: UserId("u1".into()),
            role: Role::Player,
            expires_at: Timestamp::from_second(LINK_EXPIRES_AT).unwrap(),
        })
        .await
        .unwrap();

    repos.jobs.enqueue(fixture::admin_job()).await.unwrap();
    repos.jobs.enqueue(fixture::admin_child_job()).await.unwrap();
}

async fn seeded(db_root: &Path, hash: &str) -> (Repos, Router) {
    let repos = Repos::connect(db_root).await.unwrap();
    seed(&repos, hash).await;
    let cfg = config(db_root);
    let Built { state, stream, images, trickplay, .. } =
        build_state(&repos, &cfg, &CancelRegistry::default()).unwrap();
    let capabilities = server_capabilities(CapabilityInputs {
        transcription: cfg.transcription_enabled,
        translation: cfg.translation_enabled,
        upscaling: cfg.upscaling_enabled,
        opensubtitles: false,
        tmdb: cfg.tmdb_api_key.is_some(),
        tls: false,
        webhooks: false,
        content_fetch: cfg.content_fetch_enabled,
        hardware_transcode_available: false,
        hardware_transcode_enabled: false,
    });
    (repos, app(state, stream, images, trickplay, Vec::new(), capabilities))
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
    router: Router,
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
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value =
        if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    (status, value)
}

async fn login(router: &Router, username: &str) -> (String, String) {
    let (status, body) = call(
        router.clone(),
        Method::POST,
        "/api/v1/auth/login",
        None,
        Some(json!({"username": username, "password": "pw"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    (
        body["access_token"].as_str().unwrap().to_owned(),
        body["refresh_token"].as_str().unwrap().to_owned(),
    )
}

async fn create_session(router: &Router, token: &str) -> String {
    let (status, body) = call(
        router.clone(),
        Method::POST,
        "/api/v1/sessions",
        Some(token),
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

async fn create_user(router: &Router, token: &str) -> String {
    let (status, body) = call(
        router.clone(),
        Method::POST,
        "/api/v1/users",
        Some(token),
        Some(json!({"username": "minted", "password": "pw", "role": "user"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    body["id"].as_str().unwrap().to_owned()
}

fn snapshot(name: &str, status: StatusCode, body: Value) {
    let value = json!({"status": status.as_u16(), "body": body});
    insta::assert_json_snapshot!(name, value, {
        ".body.access_token" => "[access_token]",
        ".body.refresh_token" => "[refresh_token]",
        ".body.token" => "[token]",
        ".body.expires_at" => "[expires_at]",
        ".body.code" => "[code]",
        ".body.id" => "[id]",
        ".body.created_at" => "[created_at]",
        ".body.added_at" => "[added_at]",
        ".body.updated_at" => "[updated_at]",
        ".body.session_id" => "[session_id]",
        ".body.manifest_url" => "[manifest_url]",
        ".body.version" => "[version]",
        ".body.active[].session_id" => "[session_id]",
        ".body.active[].started_at" => "[started_at]",
        ".body.active[].last_heartbeat_at" => "[last_heartbeat_at]",
    });
}

#[tokio::test]
async fn replay_request_battery() {
    let hash = password::hash("pw").unwrap();

    let setup_dir = tempfile::tempdir().unwrap();
    let (_setup, setup_app) = seeded(setup_dir.path(), &hash).await;
    let admin_access = login(&setup_app, "admin").await.0;
    let user_access = login(&setup_app, "user").await.0;

    for endpoint in requests() {
        let dir = tempfile::tempdir().unwrap();
        let (_repos, router) = seeded(dir.path(), &hash).await;

        let mut path = endpoint.path.to_owned();
        if path.contains("{session}") {
            let session = create_session(&router, &user_access).await;
            path = path.replace("{session}", &session);
        }
        if path.contains("{user}") {
            let minted = create_user(&router, &admin_access).await;
            path = path.replace("{user}", &minted);
        }

        let token = match endpoint.token {
            Token::Admin => Some(admin_access.as_str()),
            Token::User => Some(user_access.as_str()),
            Token::Anon => None,
        };

        let body = if matches!(endpoint.name, "auth_refresh" | "auth_logout") {
            let refresh = login(&router, "user").await.1;
            Some(json!({ "refresh_token": refresh }))
        } else {
            endpoint.body.clone()
        };

        let (status, response) = call(router, method(endpoint.method), &path, token, body).await;
        snapshot(endpoint.name, status, response);
    }
}

#[tokio::test]
async fn closed_pool_maps_to_internal_error() {
    let hash = password::hash("pw").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let (repos, router) = seeded(dir.path(), &hash).await;
    let token = login(&router, "user").await.0;

    repos.close().await;

    let (status, body) = call(router, Method::GET, "/api/v1/movies", Some(&token), None).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    snapshot("err_internal", status, body);
}

#[tokio::test]
async fn link_code_lifecycle_create_list_redeem_revoke() {
    let hash = password::hash("pw").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let (_repos, router) = seeded(dir.path(), &hash).await;
    let user_access = login(&router, "user").await.0;

    let (status, body) = call(
        router.clone(),
        Method::POST,
        "/api/v1/auth/link/create",
        Some(&user_access),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let code = body["code"].as_str().unwrap().to_owned();
    assert_eq!(code.len(), 8);

    let (status, body) =
        call(router.clone(), Method::GET, "/api/v1/users/u1/link-codes", Some(&user_access), None)
            .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.as_array().unwrap().iter().any(|c| c["code"].as_str() == Some(code.as_str())),
        "created code should be listed as pending"
    );

    let typed = format!("{}-{}", &code[..4], &code[4..]);
    let (status, body) = call(
        router.clone(),
        Method::POST,
        "/api/v1/auth/link",
        None,
        Some(json!({"code": typed, "device": {"name": "Roku", "platform": "roku"}})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let device_token = body["token"].as_str().unwrap().to_owned();
    assert!(device_token.starts_with("smk_"));

    let (status, _) =
        call(router.clone(), Method::GET, "/api/v1/movies", Some(&device_token), None).await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = call(
        router.clone(),
        Method::POST,
        "/api/v1/auth/link",
        None,
        Some(json!({"code": code, "device": {"name": "Dup", "platform": "roku"}})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, body) = call(
        router.clone(),
        Method::POST,
        "/api/v1/auth/link/create",
        Some(&user_access),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let code2 = body["code"].as_str().unwrap().to_owned();
    let (status, _) = call(
        router.clone(),
        Method::DELETE,
        &format!("/api/v1/users/u1/link-codes/{code2}"),
        Some(&user_access),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        router,
        Method::POST,
        "/api/v1/auth/link",
        None,
        Some(json!({"code": code2, "device": {"name": "Revoked", "platform": "roku"}})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

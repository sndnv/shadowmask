use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use api::{AppState, DiagnosticsState, diagnostics_router};
use contracts::Generator;
use domain::catalog::{MovieId, TitleId, Version, VersionId};
use domain::common::{LanguageCode, Quality};
use domain::diagnostics::{
    AccelerationMode, BenchmarkOutcome, BenchmarkRun, BenchmarkStatus, BenchmarkTarget,
    CapabilityReport, DiagnosticsStatus, FfmpegReport, HardwareReport, HardwareTest, HostReport,
    Presence, ServerDiagnostics, TranscodeBenchmark,
};
use domain::error::BenchmarkError;
use domain::library::LibraryId;
use domain::media::{AudioTrack, HdrFormat, VideoTrack};
use domain::repository::CatalogRepository;
use jiff::Timestamp;
use mocks::MockCatalogRepo;

const ADMIN: &str = "Bearer access:admin";
const USER: &str = "Bearer access:u1";
const CAPABILITIES: &str = "/api/v1/admin/server/capabilities";
const RECHECK: &str = "/api/v1/admin/server/capabilities/recheck";
const BENCHMARK: &str = "/api/v1/admin/server/benchmark";

#[derive(Clone, Default)]
struct FakeDiagnostics {
    status: Arc<Mutex<DiagnosticsStatus>>,
    rechecks: Arc<AtomicUsize>,
}

impl ServerDiagnostics for FakeDiagnostics {
    fn status(&self) -> DiagnosticsStatus {
        self.status.lock().unwrap().clone()
    }

    fn recheck(&self) {
        self.rechecks.fetch_add(1, Ordering::SeqCst);
    }
}

fn report(test: HardwareTest) -> CapabilityReport {
    CapabilityReport {
        host: HostReport {
            cpu: Some("12th Gen Intel(R) Core(TM) i3-1220P".to_owned()),
            cores: Some(10),
            threads: 12,
        },
        ffmpeg: FfmpegReport {
            version: Some("5.1.6".to_owned()),
            error: None,
            hwaccels: vec!["vaapi".to_owned()],
            encoders: vec![Presence { name: "h264_vaapi".to_owned(), present: true }],
            filters: vec![Presence { name: "tonemap_vaapi".to_owned(), present: false }],
        },
        hardware: HardwareReport {
            mode: AccelerationMode::Auto,
            device: "/dev/dri/renderD128".to_owned(),
            device_present: true,
            in_use: true,
            test,
        },
    }
}

#[derive(Clone, Default)]
struct FakeBenchmark {
    status: Arc<Mutex<BenchmarkStatus>>,
    started: Arc<Mutex<Vec<BenchmarkTarget>>>,
    busy: bool,
}

impl TranscodeBenchmark for FakeBenchmark {
    fn status(&self) -> BenchmarkStatus {
        self.status.lock().unwrap().clone()
    }

    fn start(&self, target: BenchmarkTarget) -> Result<(), BenchmarkError> {
        if self.busy {
            return Err(BenchmarkError::Running);
        }
        self.started.lock().unwrap().push(target);
        Ok(())
    }
}

fn app(diagnostics: FakeDiagnostics) -> Router {
    app_with(diagnostics, FakeBenchmark::default(), MockCatalogRepo::new())
}

fn app_with(
    diagnostics: FakeDiagnostics,
    benchmark: FakeBenchmark,
    catalog: MockCatalogRepo,
) -> Router {
    let generator = Generator::new();
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
    diagnostics_router(state, DiagnosticsState::new(diagnostics, benchmark, catalog))
}

async fn send(app: Router, method: &str, uri: &str, auth: &str) -> (StatusCode, Value) {
    send_body(app, method, uri, auth, None).await
}

async fn send_body(
    app: Router,
    method: &str,
    uri: &str,
    auth: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let builder = Request::builder().method(method).uri(uri).header(header::AUTHORIZATION, auth);
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
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[tokio::test]
async fn an_admin_reads_the_whole_report() {
    let diagnostics = FakeDiagnostics::default();
    *diagnostics.status.lock().unwrap() = DiagnosticsStatus {
        checking: false,
        checked_at: Some(Timestamp::UNIX_EPOCH),
        report: Some(report(HardwareTest::Works { low_power: true, elapsed_ms: 420 })),
    };

    let (status, body) = send(app(diagnostics), "GET", CAPABILITIES, ADMIN).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({
            "state": "ready",
            "checked_at": "1970-01-01T00:00:00Z",
            "host": { "cpu": "12th Gen Intel(R) Core(TM) i3-1220P", "cores": 10, "threads": 12 },
            "ffmpeg": {
                "version": "5.1.6",
                "error": null,
                "hwaccels": ["vaapi"],
                "encoders": [{ "name": "h264_vaapi", "present": true }],
                "filters": [{ "name": "tonemap_vaapi", "present": false }]
            },
            "hardware": {
                "mode": "auto",
                "device": "/dev/dri/renderD128",
                "device_present": true,
                "in_use": true,
                "test": {
                    "outcome": "works",
                    "low_power": true,
                    "elapsed_ms": 420,
                    "detail": null,
                    "low_power_detail": null
                }
            }
        })
    );
}

#[tokio::test]
async fn a_failed_or_skipped_test_carries_its_reasons() {
    let failed = FakeDiagnostics::default();
    *failed.status.lock().unwrap() = DiagnosticsStatus {
        checking: true,
        checked_at: None,
        report: Some(report(HardwareTest::Failed {
            detail: "no entrypoint".to_owned(),
            low_power_detail: "no low-power entrypoint".to_owned(),
        })),
    };
    let skipped = FakeDiagnostics::default();
    *skipped.status.lock().unwrap() = DiagnosticsStatus {
        checking: false,
        checked_at: None,
        report: Some(report(HardwareTest::Skipped {
            reason: "hardware acceleration is off".to_owned(),
        })),
    };

    let (_, failed) = send(app(failed), "GET", CAPABILITIES, ADMIN).await;
    let (_, skipped) = send(app(skipped), "GET", CAPABILITIES, ADMIN).await;

    assert_eq!(failed["state"], "checking", "a recheck keeps showing the last report");
    assert_eq!(
        failed["hardware"]["test"],
        json!({
            "outcome": "failed",
            "low_power": null,
            "elapsed_ms": null,
            "detail": "no entrypoint",
            "low_power_detail": "no low-power entrypoint"
        })
    );
    assert_eq!(skipped["hardware"]["test"]["outcome"], "skipped");
    assert_eq!(skipped["hardware"]["test"]["detail"], "hardware acceleration is off");
}

#[tokio::test]
async fn before_any_check_the_report_is_empty() {
    let (status, body) = send(app(FakeDiagnostics::default()), "GET", CAPABILITIES, ADMIN).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({ "state": "unchecked", "checked_at": null, "host": null, "ffmpeg": null, "hardware": null })
    );
}

#[tokio::test]
async fn an_admin_starts_a_recheck_and_is_answered_at_once() {
    let diagnostics = FakeDiagnostics::default();

    let (status, _) = send(app(diagnostics.clone()), "POST", RECHECK, ADMIN).await;

    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(diagnostics.rechecks.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn only_an_admin_sees_or_rechecks_the_capabilities() {
    let diagnostics = FakeDiagnostics::default();

    let (read, _) = send(app(diagnostics.clone()), "GET", CAPABILITIES, USER).await;
    let (recheck, _) = send(app(diagnostics.clone()), "POST", RECHECK, USER).await;

    assert_eq!(read, StatusCode::FORBIDDEN);
    assert_eq!(recheck, StatusCode::FORBIDDEN);
    assert_eq!(diagnostics.rechecks.load(Ordering::SeqCst), 0);
}

async fn film() -> MockCatalogRepo {
    let catalog = MockCatalogRepo::new();
    let id = VersionId("v1".to_owned());
    catalog.add_version(Version {
        id: id.clone(),
        title: TitleId::Movie(MovieId("m1".to_owned())),
        library: LibraryId("lib".to_owned()),
        quality: Quality::Uhd,
        container: "mkv".to_owned(),
        path: "/media/film.mkv".to_owned(),
        size_bytes: 1,
        duration_ms: 7_200_000,
        available: true,
        added_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
    });
    let video = VideoTrack {
        index: 0,
        codec: "hevc".to_owned(),
        width: 3840,
        height: 2160,
        bit_depth: 10,
        hdr: Some(HdrFormat::Hdr10),
        frame_rate: 24.0,
        bitrate: None,
    };
    let audio = AudioTrack {
        index: 1,
        codec: "eac3".to_owned(),
        channels: 6,
        language: Some(LanguageCode("en".to_owned())),
        bitrate: None,
    };
    catalog.set_version_tracks(&id, &[video], &[audio], &[], &[]).await.unwrap();
    catalog
}

fn target() -> BenchmarkTarget {
    BenchmarkTarget {
        version: VersionId("v1".to_owned()),
        path: "/media/film.mkv".to_owned(),
        duration_ms: 7_200_000,
        codec: Some("hevc".to_owned()),
        width: Some(3840),
        height: Some(2160),
        hdr: Some(HdrFormat::Hdr10),
        audio_track: Some(1),
    }
}

#[tokio::test]
async fn an_admin_benchmarks_a_version_as_the_catalog_holds_it() {
    let benchmark = FakeBenchmark::default();
    let app = app_with(FakeDiagnostics::default(), benchmark.clone(), film().await);

    let (status, _) =
        send_body(app, "POST", BENCHMARK, ADMIN, Some(json!({ "version_id": "v1" }))).await;

    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(*benchmark.started.lock().unwrap(), [target()]);
}

#[tokio::test]
async fn a_version_that_does_not_exist_cannot_be_benchmarked() {
    let benchmark = FakeBenchmark::default();
    let app = app_with(FakeDiagnostics::default(), benchmark.clone(), film().await);

    let (status, body) =
        send_body(app, "POST", BENCHMARK, ADMIN, Some(json!({ "version_id": "ghost" }))).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "not_found");
    assert!(benchmark.started.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_catalog_that_fails_answers_a_server_error() {
    let catalog = film().await;
    catalog.set_fail();
    let app = app_with(FakeDiagnostics::default(), FakeBenchmark::default(), catalog);

    let (status, _) =
        send_body(app, "POST", BENCHMARK, ADMIN, Some(json!({ "version_id": "v1" }))).await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn a_benchmark_already_running_refuses_another() {
    let benchmark = FakeBenchmark { busy: true, ..FakeBenchmark::default() };
    let app = app_with(FakeDiagnostics::default(), benchmark, film().await);

    let (status, body) =
        send_body(app, "POST", BENCHMARK, ADMIN, Some(json!({ "version_id": "v1" }))).await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "benchmark_running");
}

#[tokio::test]
async fn the_benchmark_reads_as_its_runs() {
    let benchmark = FakeBenchmark::default();
    let run = |hardware, outcome, elapsed_ms| BenchmarkRun {
        segment: 450,
        start_ms: 1_800_000,
        duration_ms: 4_000,
        hardware,
        outcome,
        elapsed_ms,
    };
    *benchmark.status.lock().unwrap() = BenchmarkStatus {
        running: true,
        target: Some(target()),
        started_at: Some(Timestamp::UNIX_EPOCH),
        finished_at: None,
        keyframes_ms: Some(1_500),
        keyframes_error: None,
        total: 6,
        runs: vec![
            run(
                true,
                BenchmarkOutcome::Failed { detail: "No usable encoding entrypoint".to_owned() },
                300,
            ),
            run(false, BenchmarkOutcome::Ok, 8_000),
            run(false, BenchmarkOutcome::TooSlow, 60_000),
        ],
    };
    let app = app_with(FakeDiagnostics::default(), benchmark, MockCatalogRepo::new());

    let (status, body) = send(app, "GET", BENCHMARK, ADMIN).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({
            "state": "running",
            "version_id": "v1",
            "started_at": "1970-01-01T00:00:00Z",
            "finished_at": null,
            "source": {
                "codec": "hevc", "width": 3840, "height": 2160, "hdr": "hdr10",
                "duration_ms": 7_200_000
            },
            "keyframes_ms": 1_500,
            "keyframes_error": null,
            "progress": { "done": 3, "total": 6 },
            "runs": [
                {
                    "segment": 450, "start_ms": 1_800_000, "duration_ms": 4_000,
                    "encoder": "vaapi", "outcome": "failed", "elapsed_ms": 300,
                    "realtime": 13.33, "detail": "No usable encoding entrypoint"
                },
                {
                    "segment": 450, "start_ms": 1_800_000, "duration_ms": 4_000,
                    "encoder": "software", "outcome": "ok", "elapsed_ms": 8_000,
                    "realtime": 0.5, "detail": null
                },
                {
                    "segment": 450, "start_ms": 1_800_000, "duration_ms": 4_000,
                    "encoder": "software", "outcome": "too_slow", "elapsed_ms": 60_000,
                    "realtime": 0.07, "detail": null
                }
            ]
        })
    );
}

#[tokio::test]
async fn a_finished_or_unrun_benchmark_says_so() {
    let finished = FakeBenchmark::default();
    *finished.status.lock().unwrap() =
        BenchmarkStatus { target: Some(target()), ..BenchmarkStatus::default() };

    let (_, idle) = send(app(FakeDiagnostics::default()), "GET", BENCHMARK, ADMIN).await;
    let (_, done) = send(
        app_with(FakeDiagnostics::default(), finished, MockCatalogRepo::new()),
        "GET",
        BENCHMARK,
        ADMIN,
    )
    .await;

    assert_eq!(idle["state"], "idle");
    assert_eq!(idle["source"], Value::Null);
    assert_eq!(idle["progress"], json!({ "done": 0, "total": 0 }));
    assert_eq!(done["state"], "done");
}

#[tokio::test]
async fn only_an_admin_reads_or_starts_a_benchmark() {
    let benchmark = FakeBenchmark::default();
    let app = || app_with(FakeDiagnostics::default(), benchmark.clone(), MockCatalogRepo::new());

    let (read, _) = send(app(), "GET", BENCHMARK, USER).await;
    let (start, _) =
        send_body(app(), "POST", BENCHMARK, USER, Some(json!({ "version_id": "v1" }))).await;

    assert_eq!(read, StatusCode::FORBIDDEN);
    assert_eq!(start, StatusCode::FORBIDDEN);
    assert!(benchmark.started.lock().unwrap().is_empty());
}

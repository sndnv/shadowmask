use axum::Json;
use axum::body::Body;
use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};

use axum::routing::get;
use axum::{Router, http::StatusCode};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use persistence::job_log::FsJobLogStore;
use serde::Serialize;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

use crate::api::Repos;
use crate::config::{Config, Loaded};
use crate::job_log_layer::JobLogLayer;

const DURATION_BUCKETS: &[f64] = &[
    10.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0, 30000.0, 60000.0, 300000.0,
    600000.0, 1800000.0, 3600000.0,
];

const VERSION: &str = env!("CARGO_PKG_VERSION");

const OWN_CRATES: [&str; 8] =
    ["server", "api", "jobs", "services", "persistence", "domain", "media", "metadata"];

fn log_directives(level: &str, sqlx_level: &str) -> String {
    let mut directives = vec!["warn".to_owned()];
    directives.extend(OWN_CRATES.iter().map(|krate| format!("{krate}={level}")));
    directives.push(format!("sqlx={sqlx_level}"));
    directives.join(",")
}

pub fn init_logging(loaded: Loaded) -> Config {
    let Loaded { config, warnings } = loaded;
    let directives = log_directives(&config.log_level, &config.sqlx_log_level);
    let console = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(directives));
    let job_logs = Targets::new()
        .with_default(LevelFilter::OFF)
        .with_targets(OWN_CRATES.iter().map(|krate| (*krate, LevelFilter::DEBUG)));
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(console))
        .with(JobLogLayer::new(FsJobLogStore::new(&config.job_log_dir)).with_filter(job_logs))
        .try_init();
    for warning in warnings {
        tracing::warn!("{warning}");
    }
    config
}

pub fn install_metrics() -> PrometheusHandle {
    let handle = PrometheusBuilder::new()
        .set_buckets(DURATION_BUCKETS)
        .expect("valid histogram buckets")
        .install_recorder()
        .expect("install prometheus recorder");
    metrics::gauge!("build_info", "version" => VERSION).set(1.0);
    handle
}

#[derive(Clone)]
struct ObsState {
    handle: PrometheusHandle,
    repos: Repos,
}

#[derive(Serialize)]
struct HealthBody {
    status: &'static str,
    version: &'static str,
}

pub fn observability_router(handle: PrometheusHandle, repos: Repos) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/health/ready", get(ready))
        .route("/metrics", get(render_metrics))
        .with_state(ObsState { handle, repos })
}

async fn health() -> Json<HealthBody> {
    Json(HealthBody { status: "ok", version: VERSION })
}

async fn ready(State(state): State<ObsState>) -> impl IntoResponse {
    if state.repos.ready().await {
        (StatusCode::OK, Json(HealthBody { status: "ok", version: VERSION }))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthBody { status: "unavailable", version: VERSION }),
        )
    }
}

async fn render_metrics(State(state): State<ObsState>) -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, "text/plain; version=0.0.4")
        .body(Body::from(state.handle.render()))
        .expect("valid metrics response")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::Request;
    use tower::ServiceExt;
    use tracing_test::traced_test;

    fn local_handle() -> PrometheusHandle {
        PrometheusBuilder::new().build_recorder().handle()
    }

    async fn repos() -> Repos {
        let dir = tempfile::tempdir().unwrap();
        Repos::connect(dir.path()).await.unwrap()
    }

    fn loaded(warnings: Vec<String>, dir: &std::path::Path) -> Loaded {
        let config = Config { job_log_dir: dir.to_path_buf(), ..Config::default() };
        Loaded { config, warnings }
    }

    #[test]
    fn init_logging_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        init_logging(loaded(Vec::new(), dir.path()));
        init_logging(loaded(Vec::new(), dir.path()));
    }

    #[traced_test]
    #[test]
    fn init_logging_says_what_loading_the_config_wanted_said() {
        let dir = tempfile::tempdir().unwrap();
        let warnings =
            vec!["[SHADOWMASK_JWT_SECRT] is not a setting".to_owned(), "second".to_owned()];

        let config = init_logging(loaded(warnings, dir.path()));

        assert_eq!(config.job_log_dir, dir.path());
        assert!(logs_contain("[SHADOWMASK_JWT_SECRT] is not a setting"));
        assert!(logs_contain("second"));
    }

    #[test]
    fn log_directives_scopes_own_crates_and_sqlx() {
        let directives = log_directives("debug", "warn");
        assert!(directives.starts_with("warn,"));
        assert!(directives.contains("services=debug"));
        assert!(directives.contains("persistence=debug"));
        assert!(directives.contains("sqlx=warn"));
        assert!(!directives.contains("sqlx=debug"));
    }

    #[test]
    fn install_metrics_renders_build_info() {
        let handle = install_metrics();
        let rendered = handle.render();
        assert!(rendered.contains("build_info"));
        assert!(rendered.contains(VERSION));
    }

    #[tokio::test]
    async fn health_returns_ok_json() {
        let repos = repos().await;
        let router = observability_router(local_handle(), repos.clone());
        let response = router
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("\"status\":\"ok\""));
        assert!(text.contains(VERSION));
        repos.close().await;
    }

    #[tokio::test]
    async fn ready_flips_to_unavailable_after_close() {
        let repos = repos().await;
        assert!(repos.ready().await);
        let router = observability_router(local_handle(), repos.clone());
        let live = router
            .clone()
            .oneshot(Request::builder().uri("/health/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(live.status(), StatusCode::OK);

        repos.close().await;
        let dead = router
            .oneshot(Request::builder().uri("/health/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(dead.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn metrics_endpoint_renders_exposition() {
        let recorder = PrometheusBuilder::new().build_recorder();
        let handle = recorder.handle();
        metrics::with_local_recorder(&recorder, || {
            metrics::gauge!("build_info", "version" => VERSION).set(1.0);
        });
        let repos = repos().await;
        let router = observability_router(handle, repos.clone());
        let response = router
            .oneshot(Request::builder().uri("/metrics").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/plain; version=0.0.4"
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(String::from_utf8_lossy(&body).contains("build_info"));
        repos.close().await;
    }
}

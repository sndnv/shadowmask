use std::time::Instant;

use axum::extract::{MatchedPath, Request, State};
use axum::http::header::{AUTHORIZATION, CONTENT_LENGTH};
use axum::middleware::Next;
use axum::response::Response;

use domain::service::AuthService;

use crate::error::ApiError;
use crate::state::AppServices;

pub async fn jwt<S>(
    State(state): State<S>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError>
where
    S: AppServices,
{
    let token = bearer(&request).ok_or_else(|| ApiError::unauthorized("missing bearer token"))?;
    let principal = state.auth().authenticate(&token).await?;
    request.extensions_mut().insert(principal);
    Ok(next.run(request).await)
}

fn bearer(request: &Request) -> Option<String> {
    request.headers().get(AUTHORIZATION)?.to_str().ok()?.strip_prefix("Bearer ").map(str::to_string)
}

pub async fn track_http(request: Request, next: Next) -> Response {
    let method = request.method().to_string();
    let endpoint = request
        .extensions()
        .get::<MatchedPath>()
        .map(|matched| matched.as_str().to_owned())
        .unwrap_or_else(|| "unmatched".to_owned());
    metrics::counter!(
        "http_requests_total",
        "method" => method.clone(),
        "endpoint" => endpoint.clone(),
    )
    .increment(1);

    let start = Instant::now();
    let response = next.run(request).await;
    let elapsed_ms = start.elapsed().as_nanos() as f64 / 1_000_000.0;

    let status = response.status().as_u16().to_string();
    metrics::counter!(
        "http_responses_total",
        "method" => method.clone(),
        "endpoint" => endpoint.clone(),
        "status" => status,
    )
    .increment(1);
    metrics::histogram!(
        "http_response_duration_milliseconds",
        "method" => method,
        "endpoint" => endpoint,
    )
    .record(elapsed_ms);
    response
}

pub async fn track_stream_bytes(request: Request, next: Next) -> Response {
    let kind = stream_kind(request.uri().path());
    let response = next.run(request).await;
    if let Some(bytes) = content_length(&response) {
        metrics::counter!("stream_bytes_sent_total", "kind" => kind).increment(bytes);
    }
    response
}

fn stream_kind(path: &str) -> &'static str {
    if path.ends_with(".m3u8") {
        "playlist"
    } else if path.ends_with("/file") {
        "file"
    } else {
        "segment"
    }
}

fn content_length(response: &Response) -> Option<u64> {
    response.headers().get(CONTENT_LENGTH)?.to_str().ok()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use std::future::Future;

    use axum::Router;
    use axum::body::Body;
    use axum::http::StatusCode;
    use axum::middleware::from_fn;
    use axum::routing::get;
    use metrics_exporter_prometheus::PrometheusBuilder;
    use tower::ServiceExt;

    use super::*;

    fn recorded<F, Fut>(work: F) -> String
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = ()>,
    {
        let recorder = PrometheusBuilder::new().build_recorder();
        let handle = recorder.handle();
        metrics::with_local_recorder(&recorder, || {
            let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
            rt.block_on(work());
        });
        handle.render()
    }

    fn request(uri: &str) -> Request {
        Request::builder().uri(uri).body(Body::empty()).unwrap()
    }

    #[test]
    fn http_middleware_records_matched_template() {
        let rendered = recorded(|| async {
            let app = Router::new()
                .route("/movies/{id}", get(|| async { "ok" }))
                .layer(from_fn(track_http));
            let response = app.oneshot(request("/movies/42")).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        });
        assert!(rendered.contains("http_requests_total"));
        assert!(rendered.contains("http_responses_total"));
        assert!(rendered.contains("http_response_duration_milliseconds"));
        assert!(rendered.contains("endpoint=\"/movies/{id}\""));
        assert!(rendered.contains("method=\"GET\""));
        assert!(rendered.contains("status=\"200\""));
    }

    #[test]
    fn http_middleware_records_nested_template() {
        let rendered = recorded(|| async {
            let inner = Router::new().route("/movies/{id}", get(|| async {}));
            let app = Router::new().nest("/api/v1", inner).layer(from_fn(track_http));
            app.oneshot(request("/api/v1/movies/7")).await.unwrap();
        });
        assert!(rendered.contains("endpoint=\"/api/v1/movies/{id}\""));
    }

    #[test]
    fn http_middleware_labels_unmatched() {
        let rendered = recorded(|| async {
            let app = Router::new().route("/known", get(|| async {})).layer(from_fn(track_http));
            let response = app.oneshot(request("/nope")).await.unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        });
        assert!(rendered.contains("endpoint=\"unmatched\""));
    }

    #[test]
    fn stream_kind_maps_paths() {
        assert_eq!(stream_kind("/stream/t/master.m3u8"), "playlist");
        assert_eq!(stream_kind("/stream/t/v0/segment1.ts"), "segment");
        assert_eq!(stream_kind("/stream/t/file"), "file");
    }

    #[test]
    fn stream_bytes_recorded_from_content_length() {
        let rendered = recorded(|| async {
            let app = Router::new()
                .route(
                    "/stream/{token}/master.m3u8",
                    get(|| async {
                        Response::builder()
                            .header(CONTENT_LENGTH, "2048")
                            .body(Body::empty())
                            .unwrap()
                    }),
                )
                .layer(from_fn(track_stream_bytes));
            app.oneshot(request("/stream/tok/master.m3u8")).await.unwrap();
        });
        assert!(rendered.contains("stream_bytes_sent_total"));
        assert!(rendered.contains("kind=\"playlist\""));
        assert!(rendered.contains("2048"));
    }

    #[test]
    fn stream_bytes_skipped_without_content_length() {
        let rendered = recorded(|| async {
            let app = Router::new()
                .route("/stream/{token}/file", get(|| async { Response::new(Body::empty()) }))
                .layer(from_fn(track_stream_bytes));
            app.oneshot(request("/stream/tok/file")).await.unwrap();
        });
        assert!(!rendered.contains("stream_bytes_sent_total"));
    }
}

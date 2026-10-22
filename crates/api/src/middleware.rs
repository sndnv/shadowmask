use std::time::Instant;

use axum::body::Body;
use axum::extract::{MatchedPath, Request, State};
use axum::http::Method;
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;
use http_body_util::BodyExt;
use tracing::warn;

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

pub(crate) async fn timed_out(
    method: Method,
    matched: Option<MatchedPath>,
    seconds: u64,
) -> ApiError {
    let route = matched.as_ref().map_or("unmatched", MatchedPath::as_str);
    warn!("Request [{method} {route}] gave up after [{seconds}s] with no answer");
    ApiError::timeout()
}

pub async fn track_stream_bytes(request: Request, next: Next) -> Response {
    let kind = stream_kind(request.uri().path());
    let response = next.run(request).await;
    let (parts, body) = response.into_parts();
    let mut sent: Option<metrics::Counter> = None;
    let body = body.map_frame(move |frame| {
        if let Some(data) = frame.data_ref() {
            let counter = sent.get_or_insert_with(
                || metrics::counter!("stream_bytes_sent_total", "kind" => kind),
            );
            counter.increment(data.len() as u64);
        }
        frame
    });
    Response::from_parts(parts, Body::new(body))
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

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::time::Duration;

    use axum::Router;
    use axum::body::Body;
    use axum::http::StatusCode;
    use axum::middleware::from_fn;
    use axum::routing::get;
    use metrics_exporter_prometheus::PrometheusBuilder;
    use tower::ServiceExt;

    use super::*;
    use crate::router::bounded;

    fn recorded<F, Fut>(work: F) -> String
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = ()>,
    {
        let recorder = PrometheusBuilder::new().build_recorder();
        let handle = recorder.handle();
        metrics::with_local_recorder(&recorder, || {
            let rt = tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap();
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
    fn http_middleware_records_a_request_that_timed_out() {
        let rendered = recorded(|| async {
            let hung = Router::new().route("/movies/{id}", get(std::future::pending::<()>));
            let app = bounded(hung, Duration::from_millis(50)).layer(from_fn(track_http));
            let response = app.oneshot(request("/movies/42")).await.unwrap();
            assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);
        });
        assert!(rendered.contains("endpoint=\"/movies/{id}\""));
        assert!(rendered.contains("status=\"504\""));
    }

    #[test]
    fn stream_kind_maps_paths() {
        assert_eq!(stream_kind("/stream/t/master.m3u8"), "playlist");
        assert_eq!(stream_kind("/stream/t/v0/segment1.ts"), "segment");
        assert_eq!(stream_kind("/stream/t/file"), "file");
    }

    fn body_of(chunks: Vec<usize>) -> Body {
        Body::from_stream(futures::stream::iter(chunks.into_iter().map(|size| {
            Ok::<_, std::convert::Infallible>(axum::body::Bytes::from(vec![0u8; size]))
        })))
    }

    fn stream_app(chunks: Vec<usize>) -> Router {
        Router::new()
            .route("/stream/{token}/file", get(move || async move { body_of(chunks) }))
            .layer(from_fn(track_stream_bytes))
    }

    #[test]
    fn stream_bytes_count_what_was_read() {
        let rendered = recorded(|| async {
            let response =
                stream_app(vec![1024, 1024]).oneshot(request("/stream/tok/file")).await.unwrap();
            let body = response.into_body().collect().await.unwrap();
            assert_eq!(body.to_bytes().len(), 2048);
        });
        assert!(rendered.contains("stream_bytes_sent_total{kind=\"file\"} 2048"));
    }

    #[test]
    fn stream_bytes_count_only_the_frames_that_were_polled() {
        let rendered = recorded(|| async {
            let response =
                stream_app(vec![1024, 1024]).oneshot(request("/stream/tok/file")).await.unwrap();
            let mut body = response.into_body();
            let frame = body.frame().await.unwrap().unwrap();
            assert_eq!(frame.data_ref().unwrap().len(), 1024);
        });
        assert!(rendered.contains("stream_bytes_sent_total{kind=\"file\"} 1024"));
        assert!(!rendered.contains("2048"));
    }

    #[test]
    fn stream_bytes_skipped_when_the_body_is_dropped() {
        let rendered = recorded(|| async {
            let response =
                stream_app(vec![1024, 1024]).oneshot(request("/stream/tok/file")).await.unwrap();
            drop(response);
        });
        assert!(!rendered.contains("stream_bytes_sent_total"));
    }

    #[test]
    fn stream_bytes_skipped_with_an_empty_body() {
        let rendered = recorded(|| async {
            let app = Router::new()
                .route("/stream/{token}/file", get(|| async { Response::new(Body::empty()) }))
                .layer(from_fn(track_stream_bytes));
            app.oneshot(request("/stream/tok/file")).await.unwrap();
        });
        assert!(!rendered.contains("stream_bytes_sent_total"));
    }
}

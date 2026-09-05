use std::path::PathBuf;

use axum::extract::{Path, Request, State};
use axum::http::HeaderValue;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use tower::ServiceExt;
use tower_http::services::ServeFile;
use tracing::{debug, warn};

use domain::session::{StreamSource, StreamTokens};

use crate::error::ApiResult;
use crate::state::StreamState;

const M3U8_CONTENT_TYPE: &str = "application/vnd.apple.mpegurl";

pub async fn master<T, G>(
    State(state): State<StreamState<T, G>>,
    Path(token): Path<String>,
) -> ApiResult<Response>
where
    T: StreamTokens + Send + Sync + 'static,
    G: StreamSource + Send + Sync + 'static,
{
    let claims = state.tokens.verify(&token)?;
    let body = state.source.master_playlist(&claims)?;
    debug!(
        "User [{}] successfully fetched stream master playlist",
        claims.user.0
    );
    Ok(([(CONTENT_TYPE, M3U8_CONTENT_TYPE)], body).into_response())
}

pub async fn media<T, G>(
    State(state): State<StreamState<T, G>>,
    Path((token, variant, file)): Path<(String, String, String)>,
    request: Request,
) -> ApiResult<Response>
where
    T: StreamTokens + Send + Sync + 'static,
    G: StreamSource + Send + Sync + 'static,
{
    let claims = state.tokens.verify(&token)?;
    let path = state
        .source
        .media_path(&claims, &variant, &file)
        .await
        .map_err(|err| {
            warn!(
                "User [{}] could not be served stream media [{variant}/{file}]: {err}",
                claims.user.0
            );
            err
        })?;
    let content_type = content_type_for(&file);
    debug!(
        "User [{}] successfully fetched stream media [{variant}/{file}]",
        claims.user.0
    );
    Ok(serve_file(path, request, content_type).await)
}

pub async fn file<T, G>(
    State(state): State<StreamState<T, G>>,
    Path(token): Path<String>,
    request: Request,
) -> ApiResult<Response>
where
    T: StreamTokens + Send + Sync + 'static,
    G: StreamSource + Send + Sync + 'static,
{
    let claims = state.tokens.verify(&token)?;
    let path = state.source.direct_file(&claims)?;
    debug!(
        "User [{}] successfully fetched direct-play file",
        claims.user.0
    );
    Ok(serve_file(path, request, None).await)
}

async fn serve_file(
    path: PathBuf,
    request: Request,
    content_type: Option<&'static str>,
) -> Response {
    let mut response = ServeFile::new(path).oneshot(request).await.into_response();
    match content_type {
        Some(content_type) if response.status().is_success() => {
            response
                .headers_mut()
                .insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
        }
        _ => {}
    }
    response
}

fn content_type_for(file: &str) -> Option<&'static str> {
    if file.ends_with(".m3u8") {
        Some(M3U8_CONTENT_TYPE)
    } else if file.ends_with(".ts") {
        Some("video/mp2t")
    } else if file.ends_with(".m4s") || file.ends_with(".mp4") {
        Some("video/mp4")
    } else if file.ends_with(".vtt") {
        Some("text/vtt")
    } else {
        None
    }
}

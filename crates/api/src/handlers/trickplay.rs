use axum::extract::{Path, Request, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use tower::ServiceExt;
use tower_http::services::ServeFile;
use tracing::debug;

use crate::error::{ApiError, ApiResult};
use crate::extract::AuthUser;
use crate::state::TrickplayState;

const JPEG_CONTENT_TYPE: &str = "image/jpeg";
const IMMUTABLE_CACHE: &str = "public, max-age=31536000, immutable";
const MAX_VERSION_ID_LEN: usize = 64;

pub async fn trickplay(
    State(state): State<TrickplayState>,
    AuthUser(_principal): AuthUser,
    Path((version_id, sheet)): Path<(String, u32)>,
    request: Request,
) -> ApiResult<Response> {
    if !is_valid_version_id(&version_id) {
        return Err(ApiError::bad_request("invalid version id"));
    }

    let etag = format!("\"{version_id}-{sheet}\"");
    let etag_value = HeaderValue::from_str(&etag).expect("etag is a valid header value");

    if request.headers().get(IF_NONE_MATCH).is_some_and(|value| value.as_bytes() == etag.as_bytes())
    {
        debug!("Trickplay [{version_id}/{sheet}] not modified");
        let mut response = StatusCode::NOT_MODIFIED.into_response();
        let headers = response.headers_mut();
        headers.insert(CACHE_CONTROL, HeaderValue::from_static(IMMUTABLE_CACHE));
        headers.insert(ETAG, etag_value);
        return Ok(response);
    }

    let path = state.root.join(&version_id).join(format!("sheet-{sheet:03}.jpg"));
    let mut response = ServeFile::new(path).oneshot(request).await.into_response();
    if response.status().is_success() {
        let headers = response.headers_mut();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static(JPEG_CONTENT_TYPE));
        headers.insert(CACHE_CONTROL, HeaderValue::from_static(IMMUTABLE_CACHE));
        headers.insert(ETAG, etag_value);
        debug!("Trickplay [{version_id}/{sheet}] served");
    }
    Ok(response)
}

fn is_valid_version_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_VERSION_ID_LEN
        && id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
}

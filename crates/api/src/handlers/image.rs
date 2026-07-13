use axum::extract::{Path, Request, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use tower::ServiceExt;
use tower_http::services::ServeFile;
use tracing::debug;

use domain::metadata::ARTWORK_WIDTHS;

use crate::error::{ApiError, ApiResult};
use crate::state::ImageState;

const PNG_CONTENT_TYPE: &str = "image/png";
const IMMUTABLE_CACHE: &str = "public, max-age=31536000, immutable";
const MAX_ARTWORK_ID_LEN: usize = 64;

pub async fn image(
    State(state): State<ImageState>,
    Path((artwork_id, width)): Path<(String, u32)>,
    request: Request,
) -> ApiResult<Response> {
    if !is_valid_artwork_id(&artwork_id) {
        return Err(ApiError::bad_request("invalid artwork id"));
    }
    if !ARTWORK_WIDTHS.contains(&width) {
        return Err(ApiError::bad_request("unsupported artwork width"));
    }

    let etag = format!("\"{artwork_id}-{width}\"");
    let etag_value = HeaderValue::from_str(&etag).expect("etag is a valid header value");

    if request
        .headers()
        .get(IF_NONE_MATCH)
        .is_some_and(|value| value.as_bytes() == etag.as_bytes())
    {
        debug!("Artwork [{artwork_id}/{width}] not modified");
        let mut response = StatusCode::NOT_MODIFIED.into_response();
        let headers = response.headers_mut();
        headers.insert(CACHE_CONTROL, HeaderValue::from_static(IMMUTABLE_CACHE));
        headers.insert(ETAG, etag_value);
        return Ok(response);
    }

    let path = state.root.join(&artwork_id).join(format!("{width}.png"));
    let mut response = ServeFile::new(path).oneshot(request).await.into_response();
    if response.status().is_success() {
        let headers = response.headers_mut();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static(PNG_CONTENT_TYPE));
        headers.insert(CACHE_CONTROL, HeaderValue::from_static(IMMUTABLE_CACHE));
        headers.insert(ETAG, etag_value);
        debug!("Artwork [{artwork_id}/{width}] served");
    }
    Ok(response)
}

fn is_valid_artwork_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ARTWORK_ID_LEN
        && id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
}

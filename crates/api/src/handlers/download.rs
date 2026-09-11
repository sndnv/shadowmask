use std::path::PathBuf;

use axum::Json;
use axum::extract::{Path, Request, State};
use axum::http::HeaderValue;
use axum::http::header::CONTENT_DISPOSITION;
use axum::response::{IntoResponse, Response};
use jiff::Timestamp;
use tower::ServiceExt;
use tower_http::services::ServeFile;
use tracing::debug;
use uuid::Uuid;

use domain::catalog::{VersionDetail, VersionId};
use domain::repository::CatalogRepository;
use domain::session::{DownloadClaims, DownloadTokens};

use crate::dto::catalog::DownloadLinkResponse;
use crate::error::{ApiError, ApiResult};
use crate::extract::AuthUser;
use domain::service::CatalogService;

use crate::state::{AppServices, DownloadState};

const LINK_TTL_SECS: i64 = 6 * 3600;

pub async fn link<S, C, D>(
    State(state): State<DownloadState<S, C, D>>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<DownloadLinkResponse>>
where
    S: AppServices,
    C: CatalogRepository + Send + Sync + 'static,
    D: DownloadTokens + Send + Sync + 'static,
{
    let version = VersionId(id);
    let detail = state.services.catalog().version(&principal, &version).await?;
    let path = downloadable_path(&principal.user.0, &detail)?;
    let expires_at = Timestamp::from_second(Timestamp::now().as_second() + LINK_TTL_SECS)
        .unwrap_or(Timestamp::MAX);
    let token = state.tokens.create(&DownloadClaims {
        user: principal.user.clone(),
        version: version.clone(),
        expires_at,
        nonce: Uuid::new_v4().to_string(),
    })?;
    debug!("User [{}] created a download link for version [{}]", principal.user.0, version.0);
    Ok(Json(DownloadLinkResponse {
        url: format!("/download/{}", token.0),
        filename: filename_of(path).to_owned(),
        size_bytes: detail.version.size_bytes,
        expires_at: expires_at.to_string(),
    }))
}

pub async fn file<S, C, D>(
    State(state): State<DownloadState<S, C, D>>,
    Path(token): Path<String>,
    request: Request,
) -> ApiResult<Response>
where
    S: AppServices,
    C: CatalogRepository + Send + Sync + 'static,
    D: DownloadTokens + Send + Sync + 'static,
{
    let claims = state.tokens.verify(&token)?;
    let detail = state
        .catalog
        .version_detail(&claims.version)
        .await
        .map_err(|_| ApiError::internal())?
        .ok_or_else(|| ApiError::not_found("version not found"))?;
    let path = downloadable_path(&claims.user.0, &detail)?;
    let disposition = content_disposition(filename_of(path));
    let mut response = ServeFile::new(PathBuf::from(path)).oneshot(request).await.into_response();
    if response.status().is_success()
        && let Ok(value) = HeaderValue::from_str(&disposition)
    {
        response.headers_mut().insert(CONTENT_DISPOSITION, value);
    }
    debug!("User [{}] downloaded version [{}]", claims.user.0, claims.version.0);
    Ok(response)
}

fn downloadable_path<'a>(user: &str, detail: &'a VersionDetail) -> Result<&'a str, ApiError> {
    if !detail.version.available || detail.version.path.is_empty() {
        debug!(
            "User [{}] failed to download version [{}]: no file on disk",
            user, detail.version.id.0
        );
        return Err(ApiError::not_found("version has no downloadable file"));
    }
    Ok(&detail.version.path)
}

fn filename_of(path: &str) -> &str {
    path.rsplit(['/', '\\']).find(|segment| !segment.is_empty()).unwrap_or("download")
}

fn content_disposition(filename: &str) -> String {
    let ascii: String =
        filename
            .chars()
            .map(|c| {
                if c.is_ascii() && c != '"' && c != '\\' && !c.is_ascii_control() { c } else { '_' }
            })
            .collect();
    format!("attachment; filename=\"{ascii}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_is_the_last_path_segment() {
        assert_eq!(filename_of("/media/movies/Big Buck Bunny.mkv"), "Big Buck Bunny.mkv");
    }

    #[test]
    fn filename_handles_windows_separators() {
        assert_eq!(filename_of(r"C:\media\clip.mp4"), "clip.mp4");
    }

    #[test]
    fn filename_ignores_trailing_separators() {
        assert_eq!(filename_of("/media/clip.mkv/"), "clip.mkv");
    }

    #[test]
    fn filename_falls_back_when_there_is_nothing_to_use() {
        assert_eq!(filename_of(""), "download");
        assert_eq!(filename_of("///"), "download");
    }

    #[test]
    fn disposition_keeps_a_plain_name() {
        assert_eq!(content_disposition("clip.mkv"), "attachment; filename=\"clip.mkv\"");
    }

    #[test]
    fn disposition_cannot_be_broken_out_of() {
        assert_eq!(
            content_disposition("a\"; drop=\"me.mkv"),
            "attachment; filename=\"a_; drop=_me.mkv\""
        );
        assert_eq!(
            content_disposition("line\r\nbreak.mkv"),
            "attachment; filename=\"line__break.mkv\""
        );
        assert_eq!(
            content_disposition(r"back\slash.mkv"),
            "attachment; filename=\"back_slash.mkv\""
        );
    }

    #[test]
    fn disposition_replaces_non_ascii() {
        assert_eq!(content_disposition("Amélie.mkv"), "attachment; filename=\"Am_lie.mkv\"");
    }
}

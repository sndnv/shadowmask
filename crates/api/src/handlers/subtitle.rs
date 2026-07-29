use std::collections::HashSet;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use tracing::debug;

use domain::catalog::VersionId;
use domain::media::{SubtitleReader, SubtitleSource, SubtitleStore, prune_orphaned_translations};
use domain::repository::CatalogRepository;

use crate::dto::catalog::SubtitleTextResponse;
use crate::error::{ApiError, ApiResult};
use crate::extract::RequireAdmin;
use crate::state::SubtitleState;

fn is_deletable(source: SubtitleSource) -> bool {
    matches!(
        source,
        SubtitleSource::OpenSubtitles
            | SubtitleSource::Generated
            | SubtitleSource::MachineTranslated
            | SubtitleSource::Combined
    )
}

pub async fn view<C, S>(
    State(state): State<SubtitleState<C, S>>,
    RequireAdmin(principal): RequireAdmin,
    Path((version, subtitle)): Path<(String, String)>,
) -> ApiResult<Json<SubtitleTextResponse>>
where
    C: CatalogRepository + Send + Sync,
    S: SubtitleReader + Send + Sync,
{
    let actor = &principal.user.0;
    let version = VersionId(version);
    let detail = state
        .catalog
        .version_detail(&version)
        .await
        .map_err(|_| ApiError::internal())?
        .ok_or_else(|| ApiError::not_found("version not found"))?;
    let file = detail
        .subtitle_files
        .iter()
        .find(|file| file.id.0 == subtitle)
        .ok_or_else(|| ApiError::not_found("subtitle not found"))?;
    let content = state
        .subtitles
        .load(&file.path)
        .await
        .map_err(|_| ApiError::internal())?;
    debug!(
        "user [{actor}] read subtitle [{subtitle}] of version [{}]",
        version.0
    );
    Ok(Json(SubtitleTextResponse { content }))
}

pub async fn delete<C, S>(
    State(state): State<SubtitleState<C, S>>,
    RequireAdmin(principal): RequireAdmin,
    Path((version, subtitle)): Path<(String, String)>,
) -> ApiResult<StatusCode>
where
    C: CatalogRepository + Send + Sync,
    S: SubtitleStore + Send + Sync,
{
    let actor = &principal.user.0;
    let version = VersionId(version);
    let detail = state
        .catalog
        .version_detail(&version)
        .await
        .map_err(|_| ApiError::internal())?
        .ok_or_else(|| ApiError::not_found("version not found"))?;
    let target = detail
        .subtitle_files
        .iter()
        .find(|file| file.id.0 == subtitle)
        .ok_or_else(|| ApiError::not_found("subtitle not found"))?;
    if !is_deletable(target.source) {
        return Err(ApiError::forbidden(
            "only downloaded or generated subtitles can be deleted",
        ));
    }

    let remaining: Vec<_> = detail
        .subtitle_files
        .iter()
        .filter(|file| file.id.0 != subtitle)
        .cloned()
        .collect();
    let kept = prune_orphaned_translations(remaining);
    let kept_ids: HashSet<&str> = kept.iter().map(|file| file.id.0.as_str()).collect();
    let removed_paths: Vec<String> = detail
        .subtitle_files
        .iter()
        .filter(|file| !kept_ids.contains(file.id.0.as_str()))
        .map(|file| file.path.clone())
        .collect();

    state
        .catalog
        .set_subtitle_files(&version, &kept)
        .await
        .map_err(|_| ApiError::internal())?;
    for path in &removed_paths {
        if let Err(err) = state.subtitles.remove(path).await {
            debug!("could not remove subtitle file [{path}]: {err}");
        }
    }
    debug!(
        "user [{actor}] deleted subtitle [{subtitle}] of version [{}]",
        version.0
    );
    Ok(StatusCode::NO_CONTENT)
}

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use tracing::debug;

use domain::catalog::VersionId;
use domain::media::SubtitleFileId;

use crate::dto::catalog::{
    CombineRequest, TranscribeRequest, TranslateRequest, UpscaleRequest, VersionResponse,
};
use crate::dto::job::JobResponse;
use crate::error::{ApiError, ApiResult};
use crate::extract::RequireAdmin;
use crate::handlers::log_fail;
use crate::pagination::{PageParams, PageResponse};
use crate::state::AppServices;

pub async fn jobs<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
) -> ApiResult<Json<Vec<JobResponse>>> {
    let actor = &principal.user.0;
    let jobs = state
        .jobs(&principal)
        .await
        .map_err(log_fail(actor, "retrieve jobs"))?;
    debug!("User [{actor}] successfully retrieved {} jobs", jobs.len());
    Ok(Json(jobs.into_iter().map(Into::into).collect()))
}

pub async fn job<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<Json<JobResponse>> {
    let actor = &principal.user.0;
    let job = state
        .jobs(&principal)
        .await
        .map_err(log_fail(actor, "retrieve job"))?
        .into_iter()
        .find(|j| j.id.0 == id)
        .ok_or_else(|| ApiError::not_found("job not found"))?;
    debug!("User [{actor}] successfully retrieved job [{id}]");
    Ok(Json(job.into()))
}

pub async fn versions<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Query(page): Query<PageParams>,
) -> ApiResult<Json<PageResponse<VersionResponse>>> {
    let actor = &principal.user.0;
    let versions = state
        .all_versions(&principal, page.to_request())
        .await
        .map_err(log_fail(actor, "retrieve versions"))?;
    debug!(
        "User [{actor}] successfully retrieved {} versions",
        versions.items.len()
    );
    Ok(Json(PageResponse::from_page(versions, |v| {
        VersionResponse::with_path(v, true)
    })))
}

pub async fn transcribe_version<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    Json(req): Json<TranscribeRequest>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let version = VersionId(id);
    state
        .trigger_transcription(
            &principal,
            &version,
            req.audio_track_index,
            req.source_language,
        )
        .await
        .map_err(log_fail(actor, "trigger transcription"))?;
    debug!(
        "User [{actor}] successfully triggered transcription for version [{}]",
        version.0
    );
    Ok(StatusCode::ACCEPTED)
}

pub async fn translate_version<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    Json(req): Json<TranslateRequest>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let version = VersionId(id);
    state
        .trigger_translation(
            &principal,
            &version,
            &SubtitleFileId(req.source_subtitle_id),
            req.target_language,
        )
        .await
        .map_err(log_fail(actor, "trigger translation"))?;
    debug!(
        "User [{actor}] successfully triggered translation for version [{}]",
        version.0
    );
    Ok(StatusCode::ACCEPTED)
}

pub async fn upscale_version<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    Json(req): Json<UpscaleRequest>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    if req.target_height == 0 {
        return Err(ApiError::bad_request("target_height must be positive"));
    }
    let version = VersionId(id);
    state
        .trigger_upscale(&principal, &version, req.target_height)
        .await
        .map_err(log_fail(actor, "trigger upscale"))?;
    debug!(
        "User [{actor}] successfully triggered upscale for version [{}]",
        version.0
    );
    Ok(StatusCode::ACCEPTED)
}

pub async fn combine_subtitles<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    Json(req): Json<CombineRequest>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    if req.primary_subtitle_id == req.secondary_subtitle_id {
        return Err(ApiError::bad_request(
            "primary and secondary subtitles must differ",
        ));
    }
    let version = VersionId(id);
    state
        .trigger_combine(
            &principal,
            &version,
            &SubtitleFileId(req.primary_subtitle_id),
            &SubtitleFileId(req.secondary_subtitle_id),
        )
        .await
        .map_err(log_fail(actor, "trigger combine"))?;
    debug!(
        "User [{actor}] successfully triggered subtitle combine for version [{}]",
        version.0
    );
    Ok(StatusCode::ACCEPTED)
}

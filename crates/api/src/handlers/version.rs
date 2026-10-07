use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use tracing::debug;

use domain::catalog::VersionId;
use domain::job::JobId;
use domain::media::SubtitleFileId;
use domain::service::{JobService, LibraryService};

use crate::dto::job::{JobStartedResponse, VersionJobResponse};
use crate::error::{ApiError, ApiResult};
use crate::extract::{RequireAdmin, RequireVersionWork};
use crate::handlers::log_fail;
use crate::state::AppServices;

type Started = (StatusCode, Json<JobStartedResponse>);

fn started(id: JobId) -> Started {
    (StatusCode::ACCEPTED, Json(id.into()))
}

#[derive(Debug, Deserialize)]
pub struct TranscribeParams {
    pub audio_track_index: Option<u32>,
    pub source_language: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TranslateParams {
    pub target_language: String,
}

#[derive(Debug, Deserialize)]
pub struct CombineParams {
    pub bottom_subtitle_id: String,
}

#[derive(Debug, Deserialize)]
pub struct UpscaleParams {
    pub target_height: u32,
}

pub async fn delete<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let version = VersionId(id);
    state
        .library()
        .delete_version(&principal, &version)
        .await
        .map_err(log_fail(actor, "delete version"))?;
    debug!("User [{actor}] deleted version [{}]", version.0);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn jobs<S: AppServices>(
    State(state): State<S>,
    RequireVersionWork(principal): RequireVersionWork,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<VersionJobResponse>>> {
    let actor = &principal.user.0;
    let version = VersionId(id);
    let jobs = state
        .job()
        .version_jobs(&principal, &version)
        .await
        .map_err(log_fail(actor, "retrieve version jobs"))?;
    debug!("User [{actor}] retrieved {} jobs of version [{}]", jobs.len(), version.0);
    Ok(Json(jobs.into_iter().map(VersionJobResponse::from).collect()))
}

pub async fn transcribe<S: AppServices>(
    State(state): State<S>,
    RequireVersionWork(principal): RequireVersionWork,
    Path(id): Path<String>,
    Query(params): Query<TranscribeParams>,
) -> ApiResult<Started> {
    let actor = &principal.user.0;
    let version = VersionId(id);
    let source_language = params.source_language.filter(|language| !language.trim().is_empty());
    let job = state
        .library()
        .trigger_transcription(&principal, &version, params.audio_track_index, source_language)
        .await
        .map_err(log_fail(actor, "trigger transcription"))?;
    debug!("User [{actor}] queued transcription job [{}] for version [{}]", job.0, version.0);
    Ok(started(job))
}

pub async fn translate<S: AppServices>(
    State(state): State<S>,
    RequireVersionWork(principal): RequireVersionWork,
    Path((id, subtitle)): Path<(String, String)>,
    Query(params): Query<TranslateParams>,
) -> ApiResult<Started> {
    let actor = &principal.user.0;
    let target_language = params.target_language.trim();
    if target_language.is_empty() {
        return Err(ApiError::bad_request("target_language is required"));
    }
    let version = VersionId(id);
    let job = state
        .library()
        .trigger_translation(
            &principal,
            &version,
            &SubtitleFileId(subtitle),
            target_language.to_owned(),
        )
        .await
        .map_err(log_fail(actor, "trigger translation"))?;
    debug!("User [{actor}] queued translation job [{}] for version [{}]", job.0, version.0);
    Ok(started(job))
}

pub async fn combine<S: AppServices>(
    State(state): State<S>,
    RequireVersionWork(principal): RequireVersionWork,
    Path((id, top)): Path<(String, String)>,
    Query(params): Query<CombineParams>,
) -> ApiResult<Started> {
    let actor = &principal.user.0;
    if top == params.bottom_subtitle_id {
        return Err(ApiError::bad_request("top and bottom subtitles must differ"));
    }
    let version = VersionId(id);
    let job = state
        .library()
        .trigger_combine(
            &principal,
            &version,
            &SubtitleFileId(top),
            &SubtitleFileId(params.bottom_subtitle_id),
        )
        .await
        .map_err(log_fail(actor, "trigger combine"))?;
    debug!("User [{actor}] queued subtitle combine job [{}] for version [{}]", job.0, version.0);
    Ok(started(job))
}

pub async fn upscale<S: AppServices>(
    State(state): State<S>,
    RequireVersionWork(principal): RequireVersionWork,
    Path(id): Path<String>,
    Query(params): Query<UpscaleParams>,
) -> ApiResult<Started> {
    let actor = &principal.user.0;
    if params.target_height == 0 {
        return Err(ApiError::bad_request("target_height must be positive"));
    }
    let version = VersionId(id);
    let job = state
        .library()
        .trigger_upscale(&principal, &version, params.target_height)
        .await
        .map_err(log_fail(actor, "trigger upscale"))?;
    debug!("User [{actor}] queued upscale job [{}] for version [{}]", job.0, version.0);
    Ok(started(job))
}

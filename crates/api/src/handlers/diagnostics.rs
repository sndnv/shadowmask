use axum::extract::State;
use axum::http::StatusCode;
use domain::catalog::VersionId;
use domain::diagnostics::{
    BenchmarkTarget, PlaybackPrepAudit, ServerDiagnostics, TranscodeBenchmark,
};
use domain::repository::CatalogRepository;
use tracing::debug;

use crate::dto::server::{
    BenchmarkResponse, CapabilitiesResponse, PlaybackPrepResponse, StartBenchmarkRequest,
};
use crate::error::{ApiError, ApiResult};
use crate::extract::{Json, RequireAdmin};
use crate::handlers::log_fail;
use crate::state::DiagnosticsState;

pub async fn capabilities<D: ServerDiagnostics, B, C, P>(
    State(state): State<DiagnosticsState<D, B, C, P>>,
    RequireAdmin(principal): RequireAdmin,
) -> Json<CapabilitiesResponse> {
    let status = state.diagnostics.status();
    debug!("User [{}] retrieved the server capabilities", principal.user.0);
    Json(status.into())
}

pub async fn recheck<D: ServerDiagnostics, B, C, P>(
    State(state): State<DiagnosticsState<D, B, C, P>>,
    RequireAdmin(principal): RequireAdmin,
) -> StatusCode {
    state.diagnostics.recheck();
    debug!("User [{}] asked for the server capabilities to be checked again", principal.user.0);
    StatusCode::ACCEPTED
}

pub async fn benchmark<D, B: TranscodeBenchmark, C, P>(
    State(state): State<DiagnosticsState<D, B, C, P>>,
    RequireAdmin(principal): RequireAdmin,
) -> Json<BenchmarkResponse> {
    let status = state.benchmark.status();
    debug!("User [{}] retrieved the transcode benchmark", principal.user.0);
    Json(status.into())
}

pub async fn start_benchmark<D, B: TranscodeBenchmark, C: CatalogRepository + Send + Sync, P>(
    State(state): State<DiagnosticsState<D, B, C, P>>,
    RequireAdmin(principal): RequireAdmin,
    Json(request): Json<StartBenchmarkRequest>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let version = VersionId(request.version_id);
    let detail = state
        .catalog
        .version_detail(&version)
        .await
        .map_err(log_fail(actor, "look up the version to benchmark"))
        .map_err(|_| ApiError::internal())?
        .ok_or_else(|| ApiError::not_found("version not found"))?;
    let keyframes_us = state
        .catalog
        .keyframes_us(&version)
        .await
        .map_err(log_fail(actor, "read the keyframes of the version to benchmark"))
        .map_err(|_| ApiError::internal())?;
    state
        .benchmark
        .start(BenchmarkTarget::of(&detail, keyframes_us))
        .map_err(log_fail(actor, "start a transcode benchmark"))?;
    debug!("User [{actor}] started a transcode benchmark of version [{}]", version.0);
    Ok(StatusCode::ACCEPTED)
}

pub async fn playback_prep<D: ServerDiagnostics, B, C, P: PlaybackPrepAudit>(
    State(state): State<DiagnosticsState<D, B, C, P>>,
    RequireAdmin(principal): RequireAdmin,
) -> ApiResult<Json<PlaybackPrepResponse>> {
    let actor = &principal.user.0;
    let ffmpeg = state.diagnostics.status().report.and_then(|report| report.ffmpeg.version);
    let libraries = state
        .prep
        .libraries(ffmpeg.as_deref())
        .await
        .map_err(log_fail(actor, "look up which libraries need their files read again"))
        .map_err(|_| ApiError::internal())?;
    debug!("User [{actor}] retrieved the playback preparation of [{}] libraries", libraries.len());
    Ok(Json(PlaybackPrepResponse::new(ffmpeg, libraries)))
}

use axum::extract::State;
use axum::http::StatusCode;
use domain::catalog::VersionId;
use domain::diagnostics::{BenchmarkTarget, ServerDiagnostics, TranscodeBenchmark};
use domain::repository::CatalogRepository;
use tracing::debug;

use crate::dto::server::{BenchmarkResponse, CapabilitiesResponse, StartBenchmarkRequest};
use crate::error::{ApiError, ApiResult};
use crate::extract::{Json, RequireAdmin};
use crate::handlers::log_fail;
use crate::state::DiagnosticsState;

pub async fn capabilities<D: ServerDiagnostics, B, C>(
    State(state): State<DiagnosticsState<D, B, C>>,
    RequireAdmin(principal): RequireAdmin,
) -> Json<CapabilitiesResponse> {
    let status = state.diagnostics.status();
    debug!("User [{}] retrieved the server capabilities", principal.user.0);
    Json(status.into())
}

pub async fn recheck<D: ServerDiagnostics, B, C>(
    State(state): State<DiagnosticsState<D, B, C>>,
    RequireAdmin(principal): RequireAdmin,
) -> StatusCode {
    state.diagnostics.recheck();
    debug!("User [{}] asked for the server capabilities to be checked again", principal.user.0);
    StatusCode::ACCEPTED
}

pub async fn benchmark<D, B: TranscodeBenchmark, C>(
    State(state): State<DiagnosticsState<D, B, C>>,
    RequireAdmin(principal): RequireAdmin,
) -> Json<BenchmarkResponse> {
    let status = state.benchmark.status();
    debug!("User [{}] retrieved the transcode benchmark", principal.user.0);
    Json(status.into())
}

pub async fn start_benchmark<D, B: TranscodeBenchmark, C: CatalogRepository + Send + Sync>(
    State(state): State<DiagnosticsState<D, B, C>>,
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
    state
        .benchmark
        .start(BenchmarkTarget::of(&detail))
        .map_err(log_fail(actor, "start a transcode benchmark"))?;
    debug!("User [{actor}] started a transcode benchmark of version [{}]", version.0);
    Ok(StatusCode::ACCEPTED)
}

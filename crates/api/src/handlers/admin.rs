use axum::Json;
use axum::extract::{Path, State};
use tracing::debug;

use crate::dto::job::JobResponse;
use crate::error::{ApiError, ApiResult};
use crate::extract::RequireAdmin;
use crate::handlers::log_fail;
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

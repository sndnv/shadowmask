use axum::extract::{Path, State};
use tracing::debug;

use domain::library::{CancelScope, LibraryId};
use domain::service::MaintenanceService;
use domain::user::Principal;

use crate::dto::library::{
    CancelJobsRequest, CancelledJobsResponse, CleanRequest, CleanResponse, PurgeRequest,
    PurgeResponse,
};
use crate::error::ApiResult;
use crate::extract::{Json, RequireAdmin};
use crate::handlers::{log_fail, to_completion};
use crate::state::MaintenanceState;

pub async fn purge<M: MaintenanceService + Send + Sync + 'static>(
    State(state): State<MaintenanceState<M>>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    body: Option<Json<PurgeRequest>>,
) -> ApiResult<Json<PurgeResponse>> {
    let wipe_user_data = body.is_some_and(|Json(request)| request.wipe_user_data);
    to_completion(purged(state, principal, LibraryId(id), wipe_user_data)).await
}

async fn purged<M: MaintenanceService + Send + Sync>(
    state: MaintenanceState<M>,
    principal: Principal,
    id: LibraryId,
    wipe_user_data: bool,
) -> ApiResult<Json<PurgeResponse>> {
    let actor = &principal.user.0;
    let report = state
        .service
        .purge_library(&principal, &id, wipe_user_data)
        .await
        .map_err(log_fail(actor, "purge library"))?;
    debug!("User [{actor}] purged library [{}], user data wiped: [{wipe_user_data}]", id.0);
    Ok(Json(report.into()))
}

pub async fn clean<M: MaintenanceService + Send + Sync + 'static>(
    State(state): State<MaintenanceState<M>>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    body: Option<Json<CleanRequest>>,
) -> ApiResult<Json<CleanResponse>> {
    let wipe_user_data = body.is_some_and(|Json(request)| request.wipe_user_data);
    to_completion(cleaned(state, principal, Some(LibraryId(id)), wipe_user_data)).await
}

pub async fn clean_all<M: MaintenanceService + Send + Sync + 'static>(
    State(state): State<MaintenanceState<M>>,
    RequireAdmin(principal): RequireAdmin,
    body: Option<Json<CleanRequest>>,
) -> ApiResult<Json<CleanResponse>> {
    let wipe_user_data = body.is_some_and(|Json(request)| request.wipe_user_data);
    to_completion(cleaned(state, principal, None, wipe_user_data)).await
}

async fn cleaned<M: MaintenanceService + Send + Sync>(
    state: MaintenanceState<M>,
    principal: Principal,
    id: Option<LibraryId>,
    wipe_user_data: bool,
) -> ApiResult<Json<CleanResponse>> {
    let actor = &principal.user.0;
    let Some(id) = id else {
        let report = state
            .service
            .clean_all(&principal, wipe_user_data)
            .await
            .map_err(log_fail(actor, "clean every library"))?;
        debug!("User [{actor}] cleaned every library, user data wiped: [{wipe_user_data}]");
        return Ok(Json(report.into()));
    };
    let report = state
        .service
        .clean_library(&principal, &id, wipe_user_data)
        .await
        .map_err(log_fail(actor, "clean library"))?;
    debug!("User [{actor}] cleaned library [{}], user data wiped: [{wipe_user_data}]", id.0);
    Ok(Json(report.into()))
}

pub async fn cancel_jobs<M: MaintenanceService + Send + Sync + 'static>(
    State(state): State<MaintenanceState<M>>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    body: Option<Json<CancelJobsRequest>>,
) -> ApiResult<Json<CancelledJobsResponse>> {
    let actor = &principal.user.0;
    let id = LibraryId(id);
    let scope = body.map_or(CancelScope::All, |Json(request)| request.scope());
    let outcome = state
        .service
        .cancel_library_jobs(&principal, &id, scope)
        .await
        .map_err(log_fail(actor, "cancel the jobs of library"))?;
    debug!("User [{actor}] cancelled the jobs of library [{}], scope: [{scope:?}]", id.0);
    Ok(Json(outcome.into()))
}

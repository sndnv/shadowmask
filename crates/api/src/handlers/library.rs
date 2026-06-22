use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use tracing::debug;

use domain::library::LibraryId;

use crate::dto::catalog::VersionResponse;
use crate::dto::library::{
    DuplicateCandidateResponse, LibraryResponse, ScanStateResponse, UnmatchedFileResponse,
};
use crate::error::ApiResult;
use crate::extract::{AuthUser, RequireAdmin};
use crate::handlers::log_fail;
use crate::state::AppServices;

pub async fn libraries<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
) -> ApiResult<Json<Vec<LibraryResponse>>> {
    let actor = &principal.user.0;
    let libraries = state
        .libraries()
        .await
        .map_err(log_fail(actor, "retrieve libraries"))?;
    debug!(
        "User [{actor}] successfully retrieved {} libraries",
        libraries.len()
    );
    Ok(Json(libraries.into_iter().map(Into::into).collect()))
}

pub async fn library<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<LibraryResponse>> {
    let actor = &principal.user.0;
    let id = LibraryId(id);
    let library = state
        .library(&id)
        .await
        .map_err(log_fail(actor, "retrieve library"))?;
    debug!("User [{actor}] successfully retrieved library [{}]", id.0);
    Ok(Json(library.into()))
}

pub async fn scan_state<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<Json<ScanStateResponse>> {
    let actor = &principal.user.0;
    let id = LibraryId(id);
    let scan = state
        .scan_state(&id)
        .await
        .map_err(log_fail(actor, "retrieve scan state"))?;
    debug!(
        "User [{actor}] successfully retrieved scan state for library [{}]",
        id.0
    );
    Ok(Json(scan.into()))
}

pub async fn trigger_scan<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let id = LibraryId(id);
    state
        .trigger_scan(&id)
        .await
        .map_err(log_fail(actor, "trigger scan"))?;
    debug!(
        "User [{actor}] successfully triggered scan for library [{}]",
        id.0
    );
    Ok(StatusCode::ACCEPTED)
}

pub async fn unmatched<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<UnmatchedFileResponse>>> {
    let actor = &principal.user.0;
    let id = LibraryId(id);
    let unmatched = state
        .unmatched(&id)
        .await
        .map_err(log_fail(actor, "retrieve unmatched files"))?;
    debug!(
        "User [{actor}] successfully retrieved {} unmatched files for library [{}]",
        unmatched.len(),
        id.0
    );
    Ok(Json(unmatched.into_iter().map(Into::into).collect()))
}

pub async fn duplicates<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<DuplicateCandidateResponse>>> {
    let actor = &principal.user.0;
    let id = LibraryId(id);
    let duplicates = state
        .duplicates(&id)
        .await
        .map_err(log_fail(actor, "retrieve duplicate candidates"))?;
    debug!(
        "User [{actor}] successfully retrieved {} duplicate candidates for library [{}]",
        duplicates.len(),
        id.0
    );
    Ok(Json(duplicates.into_iter().map(Into::into).collect()))
}

pub async fn versions<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<VersionResponse>>> {
    let actor = &principal.user.0;
    let id = LibraryId(id);
    let versions = state
        .library_versions(&id)
        .await
        .map_err(log_fail(actor, "retrieve library versions"))?;
    debug!(
        "User [{actor}] successfully retrieved {} versions for library [{}]",
        versions.len(),
        id.0
    );
    Ok(Json(versions.into_iter().map(Into::into).collect()))
}

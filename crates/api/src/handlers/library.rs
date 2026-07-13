use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use tracing::debug;

use domain::library::{DuplicateCandidateId, LibraryId, UnmatchedFileId};

use crate::dto::catalog::VersionResponse;
use crate::dto::library::{
    CreateLibraryRequest, DuplicateCandidateResponse, LibraryResponse, ResolveCandidateResponse,
    ResolveUnmatchedRequest, ScanStateResponse, UnmatchedFileResponse, UpdateLibraryRequest,
};
use crate::error::ApiResult;
use crate::extract::{AuthUser, RequireAdmin};
use crate::handlers::log_fail;
use crate::pagination::{PageParams, PageResponse};
use crate::state::AppServices;

pub async fn libraries<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
) -> ApiResult<Json<Vec<LibraryResponse>>> {
    let actor = &principal.user.0;
    let libraries = state
        .libraries(&principal)
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
        .library(&principal, &id)
        .await
        .map_err(log_fail(actor, "retrieve library"))?;
    debug!("User [{actor}] successfully retrieved library [{}]", id.0);
    Ok(Json(library.into()))
}

pub async fn create_library<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Json(req): Json<CreateLibraryRequest>,
) -> ApiResult<(StatusCode, Json<LibraryResponse>)> {
    let actor = &principal.user.0;
    let library = state
        .create_library(&principal, req.into())
        .await
        .map_err(log_fail(actor, "create library"))?;
    debug!(
        "User [{actor}] successfully created library [{}]",
        library.id.0
    );
    Ok((StatusCode::CREATED, Json(library.into())))
}

pub async fn update_library<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    Json(req): Json<UpdateLibraryRequest>,
) -> ApiResult<Json<LibraryResponse>> {
    let actor = &principal.user.0;
    let id = LibraryId(id);
    let library = state
        .update_library(&principal, &id, req.into())
        .await
        .map_err(log_fail(actor, "update library"))?;
    debug!("User [{actor}] successfully updated library [{}]", id.0);
    Ok(Json(library.into()))
}

pub async fn delete_library<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let id = LibraryId(id);
    state
        .delete_library(&principal, &id)
        .await
        .map_err(log_fail(actor, "delete library"))?;
    debug!("User [{actor}] successfully deleted library [{}]", id.0);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn scan_state<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<Json<ScanStateResponse>> {
    let actor = &principal.user.0;
    let id = LibraryId(id);
    let scan = state
        .scan_state(&principal, &id)
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
        .trigger_scan(&principal, &id)
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
    Query(page): Query<PageParams>,
) -> ApiResult<Json<PageResponse<UnmatchedFileResponse>>> {
    let actor = &principal.user.0;
    let id = LibraryId(id);
    let unmatched = state
        .unmatched(&principal, &id, page.to_request())
        .await
        .map_err(log_fail(actor, "retrieve unmatched files"))?;
    debug!(
        "User [{actor}] successfully retrieved {} unmatched files for library [{}]",
        unmatched.items.len(),
        id.0
    );
    Ok(Json(PageResponse::from_page(
        unmatched,
        UnmatchedFileResponse::from,
    )))
}

pub async fn duplicates<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    Query(page): Query<PageParams>,
) -> ApiResult<Json<PageResponse<DuplicateCandidateResponse>>> {
    let actor = &principal.user.0;
    let id = LibraryId(id);
    let duplicates = state
        .duplicates(&principal, &id, page.to_request())
        .await
        .map_err(log_fail(actor, "retrieve duplicate candidates"))?;
    debug!(
        "User [{actor}] successfully retrieved {} duplicate candidates for library [{}]",
        duplicates.items.len(),
        id.0
    );
    Ok(Json(PageResponse::from_page(
        duplicates,
        DuplicateCandidateResponse::from,
    )))
}

#[derive(Debug, Deserialize)]
pub struct CandidateParams {
    pub q: Option<String>,
}

pub async fn unmatched_candidates<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path((id, uid)): Path<(String, String)>,
    Query(params): Query<CandidateParams>,
) -> ApiResult<Json<Vec<ResolveCandidateResponse>>> {
    let actor = &principal.user.0;
    let candidates = state
        .unmatched_candidates(&principal, &LibraryId(id), &UnmatchedFileId(uid), params.q)
        .await
        .map_err(log_fail(actor, "retrieve unmatched candidates"))?;
    debug!(
        "User [{actor}] successfully retrieved {} candidates for an unmatched file",
        candidates.len()
    );
    Ok(Json(candidates.into_iter().map(Into::into).collect()))
}

pub async fn resolve_unmatched<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path((id, uid)): Path<(String, String)>,
    Json(req): Json<ResolveUnmatchedRequest>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    state
        .resolve_unmatched(
            &principal,
            &LibraryId(id),
            &UnmatchedFileId(uid),
            req.target.into(),
        )
        .await
        .map_err(log_fail(actor, "resolve unmatched file"))?;
    debug!("User [{actor}] successfully resolved an unmatched file");
    Ok(StatusCode::ACCEPTED)
}

pub async fn dismiss_duplicate<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path((id, did)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    state
        .dismiss_duplicate(&principal, &LibraryId(id), &DuplicateCandidateId(did))
        .await
        .map_err(log_fail(actor, "dismiss duplicate candidate"))?;
    debug!("User [{actor}] successfully dismissed a duplicate candidate");
    Ok(StatusCode::NO_CONTENT)
}

pub async fn resolve_duplicate<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path((id, did)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    state
        .resolve_duplicate(&principal, &LibraryId(id), &DuplicateCandidateId(did))
        .await
        .map_err(log_fail(actor, "resolve duplicate candidate"))?;
    debug!("User [{actor}] successfully resolved a duplicate candidate");
    Ok(StatusCode::NO_CONTENT)
}

pub async fn versions<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    Query(page): Query<PageParams>,
) -> ApiResult<Json<PageResponse<VersionResponse>>> {
    let actor = &principal.user.0;
    let id = LibraryId(id);
    let versions = state
        .library_versions(&principal, &id, page.to_request())
        .await
        .map_err(log_fail(actor, "retrieve library versions"))?;
    debug!(
        "User [{actor}] successfully retrieved {} versions for library [{}]",
        versions.items.len(),
        id.0
    );
    Ok(Json(PageResponse::from_page(
        versions,
        VersionResponse::from,
    )))
}

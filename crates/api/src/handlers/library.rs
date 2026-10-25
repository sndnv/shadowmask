use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use tracing::debug;

use domain::library::{DuplicateCandidateId, LibraryId, ResolveTarget, ScanMode, UnmatchedFileId};
use domain::user::Principal;

use crate::dto::catalog::VersionResponse;
use crate::dto::library::{
    CreateLibraryRequest, DuplicateCandidateResponse, LibraryResponse, ResolveCandidateResponse,
    ResolveUnmatchedRequest, ScanRequest, ScanStateResponse, UnmatchedFileResponse,
    UpdateLibraryRequest,
};
use crate::error::ApiResult;
use crate::extract::{AuthUser, Json, RequireAdmin};
use crate::handlers::{log_fail, to_completion};
use crate::pagination::{PageParams, PageResponse};
use domain::service::{CatalogService, LibraryService};

use crate::state::AppServices;

pub async fn libraries<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
) -> ApiResult<Json<Vec<LibraryResponse>>> {
    let actor = &principal.user.0;
    let libraries = state
        .library()
        .libraries(&principal)
        .await
        .map_err(log_fail(actor, "retrieve libraries"))?;
    debug!("User [{actor}] retrieved {} libraries", libraries.len());
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
        .library()
        .library(&principal, &id)
        .await
        .map_err(log_fail(actor, "retrieve library"))?;
    debug!("User [{actor}] retrieved library [{}]", id.0);
    Ok(Json(library.into()))
}

pub async fn create_library<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Json(req): Json<CreateLibraryRequest>,
) -> ApiResult<(StatusCode, Json<LibraryResponse>)> {
    to_completion(create(state, principal, req)).await
}

async fn create<S: AppServices>(
    state: S,
    principal: Principal,
    req: CreateLibraryRequest,
) -> ApiResult<(StatusCode, Json<LibraryResponse>)> {
    let actor = &principal.user.0;
    let library = state
        .library()
        .create_library(&principal, req.into())
        .await
        .map_err(log_fail(actor, "create library"))?;
    debug!("User [{actor}] created library [{}]", library.id.0);
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
        .library()
        .update_library(&principal, &id, req.into())
        .await
        .map_err(log_fail(actor, "update library"))?;
    debug!("User [{actor}] updated library [{}]", id.0);
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
        .library()
        .delete_library(&principal, &id)
        .await
        .map_err(log_fail(actor, "delete library"))?;
    debug!("User [{actor}] deleted library [{}]", id.0);
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
        .library()
        .scan_state(&principal, &id)
        .await
        .map_err(log_fail(actor, "retrieve scan state"))?;
    debug!("User [{actor}] retrieved scan state for library [{}]", id.0);
    Ok(Json(scan.into()))
}

pub async fn trigger_scan<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    body: Option<Json<ScanRequest>>,
) -> ApiResult<StatusCode> {
    let mode = body.map_or(ScanMode::Normal, |Json(req)| req.mode());
    to_completion(scan(state, principal, LibraryId(id), mode)).await
}

async fn scan<S: AppServices>(
    state: S,
    principal: Principal,
    id: LibraryId,
    mode: ScanMode,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    state
        .library()
        .trigger_scan(&principal, &id, mode)
        .await
        .map_err(log_fail(actor, "trigger scan"))?;
    debug!("User [{actor}] triggered a [{mode:?}] scan for library [{}]", id.0);
    Ok(StatusCode::ACCEPTED)
}

pub async fn refresh_metadata<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let id = LibraryId(id);
    state
        .library()
        .refresh_library_metadata(&principal, &id)
        .await
        .map_err(log_fail(actor, "refresh library metadata"))?;
    debug!("User [{actor}] queued a metadata refresh for library [{}]", id.0);
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
    let files = state
        .library()
        .unmatched(&principal, &id, page.to_request())
        .await
        .map_err(log_fail(actor, "retrieve unmatched files"))?;
    debug!("User [{actor}] retrieved {} unmatched files for library [{}]", files.items.len(), id.0);
    Ok(Json(PageResponse::from_page(files, UnmatchedFileResponse::from)))
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
        .library()
        .duplicates(&principal, &id, page.to_request())
        .await
        .map_err(log_fail(actor, "retrieve duplicate candidates"))?;
    debug!("User [{actor}] retrieved {} duplicates for library [{}]", duplicates.items.len(), id.0);
    Ok(Json(PageResponse::from_page(duplicates, DuplicateCandidateResponse::from)))
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
    let file = UnmatchedFileId(uid);
    let candidates = state
        .library()
        .unmatched_candidates(&principal, &LibraryId(id), &file, params.q)
        .await
        .map_err(log_fail(actor, "retrieve unmatched candidates"))?;
    #[rustfmt::skip]
    debug!("User [{actor}] retrieved {} candidates for unmatched file [{}]", candidates.len(), file.0);
    Ok(Json(candidates.into_iter().map(Into::into).collect()))
}

pub async fn resolve_unmatched<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path((id, uid)): Path<(String, String)>,
    Json(req): Json<ResolveUnmatchedRequest>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let file = UnmatchedFileId(uid);
    let target = ResolveTarget::from(req.target);
    state
        .library()
        .resolve_unmatched(&principal, &LibraryId(id), &file, target.clone())
        .await
        .map_err(log_fail(actor, "resolve unmatched file"))?;
    debug!("User [{actor}] resolved unmatched file [{}] to [{target:?}]", file.0);
    Ok(StatusCode::ACCEPTED)
}

pub async fn dismiss_duplicate<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path((id, did)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let candidate = DuplicateCandidateId(did);
    state
        .library()
        .dismiss_duplicate(&principal, &LibraryId(id), &candidate)
        .await
        .map_err(log_fail(actor, "dismiss duplicate candidate"))?;
    debug!("User [{actor}] dismissed duplicate candidate [{}]", candidate.0);
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
        .catalog()
        .library_versions(&principal, &id, page.to_request())
        .await
        .map_err(log_fail(actor, "retrieve library versions"))?;
    debug!("User [{actor}] retrieved {} versions for library [{}]", versions.items.len(), id.0);
    Ok(Json(PageResponse::from_page(versions, |v| VersionResponse::with_path(v, true))))
}

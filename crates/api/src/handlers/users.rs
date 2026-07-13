use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use tracing::debug;

use domain::library::LibraryId;
use domain::user::UserId;

use crate::dto::session::PlaybackSessionResponse;
use crate::dto::user::{
    ChangePasswordRequest, CreateUserRequest, LibraryAccessResponse, SetLibraryAccessRequest,
    UpdateProfileRequest, UserResponse,
};
use crate::error::ApiResult;
use crate::extract::{AuthUser, RequireAdmin};
use crate::handlers::{log_fail, require_admin_or_self};
use crate::pagination::{PageParams, PageResponse};
use crate::state::AppServices;

pub async fn list<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Query(page): Query<PageParams>,
) -> ApiResult<Json<PageResponse<UserResponse>>> {
    let actor = &principal.user.0;
    let users = state
        .list(page.to_request())
        .await
        .map_err(log_fail(actor, "list users"))?;
    debug!(
        "User [{actor}] successfully listed {} users",
        users.items.len()
    );
    Ok(Json(PageResponse::from_page(users, UserResponse::from)))
}

pub async fn create<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Json(req): Json<CreateUserRequest>,
) -> ApiResult<(StatusCode, Json<UserResponse>)> {
    let actor = &principal.user.0;
    let user = state
        .create(req.into())
        .await
        .map_err(log_fail(actor, "create a user"))?;
    debug!("User [{actor}] successfully created user [{}]", user.id.0);
    Ok((StatusCode::CREATED, Json(user.into())))
}

pub async fn activity<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Query(page): Query<PageParams>,
) -> ApiResult<Json<PageResponse<PlaybackSessionResponse>>> {
    let actor = &principal.user.0;
    let sessions = state
        .active_sessions(&principal, page.to_request())
        .await
        .map_err(log_fail(actor, "retrieve active sessions"))?;
    let total = sessions.total;
    let offset = sessions.offset;
    let limit = sessions.limit;
    let now_playing = state
        .now_playing(sessions.items)
        .await
        .map_err(log_fail(actor, "retrieve active sessions"))?;
    debug!(
        "User [{actor}] successfully retrieved {} active sessions",
        now_playing.len()
    );
    Ok(Json(PageResponse {
        items: now_playing
            .into_iter()
            .map(PlaybackSessionResponse::from)
            .collect(),
        total,
        offset,
        limit,
    }))
}

pub async fn get<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<UserResponse>> {
    let actor = &principal.user.0;
    let target = UserId(id);
    require_admin_or_self(&principal, &target)?;
    let user = state
        .get(&target)
        .await
        .map_err(log_fail(actor, "retrieve a user"))?;
    debug!("User [{actor}] successfully retrieved user [{}]", target.0);
    Ok(Json(user.into()))
}

pub async fn current<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
) -> ApiResult<Json<UserResponse>> {
    let actor = &principal.user.0;
    let user = state
        .get(&principal.user)
        .await
        .map_err(log_fail(actor, "retrieve current user"))?;
    debug!("User [{actor}] successfully retrieved self");
    Ok(Json(user.into()))
}

pub async fn update_profile<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateProfileRequest>,
) -> ApiResult<Json<UserResponse>> {
    let actor = &principal.user.0;
    let target = UserId(id);
    require_admin_or_self(&principal, &target)?;
    let user = state
        .update_profile(&target, req.into())
        .await
        .map_err(log_fail(actor, "update a user"))?;
    debug!("User [{actor}] successfully updated user [{}]", target.0);
    Ok(Json(user.into()))
}

pub async fn delete<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let target = UserId(id);
    state
        .delete(&target)
        .await
        .map_err(log_fail(actor, "delete a user"))?;
    debug!("User [{actor}] successfully deleted user [{}]", target.0);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn library_access<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<LibraryAccessResponse>>> {
    let actor = &principal.user.0;
    let target = UserId(id);
    let access = state
        .library_access(&target)
        .await
        .map_err(log_fail(actor, "retrieve library access"))?;
    debug!(
        "User [{actor}] successfully retrieved {} library access entries for user [{}]",
        access.len(),
        target.0
    );
    Ok(Json(access.into_iter().map(Into::into).collect()))
}

pub async fn change_password<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
    Json(req): Json<ChangePasswordRequest>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let target = UserId(id);
    require_admin_or_self(&principal, &target)?;
    state
        .change_password(
            &principal,
            &target,
            req.current_password.as_deref(),
            &req.new_password,
        )
        .await
        .map_err(log_fail(actor, "change password"))?;
    state
        .logout_all(&target)
        .await
        .map_err(log_fail(actor, "revoke sessions after password change"))?;
    debug!(
        "User [{actor}] successfully changed password for user [{}]",
        target.0
    );
    Ok(StatusCode::NO_CONTENT)
}

pub async fn set_library_access<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    Json(req): Json<SetLibraryAccessRequest>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let target = UserId(id);
    let libraries: Vec<LibraryId> = req.libraries.into_iter().map(LibraryId).collect();
    state
        .set_library_access(&target, &libraries)
        .await
        .map_err(log_fail(actor, "set library access"))?;
    debug!(
        "User [{actor}] successfully set library access for user [{}]",
        target.0
    );
    Ok(StatusCode::NO_CONTENT)
}

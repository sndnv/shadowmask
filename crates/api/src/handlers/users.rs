use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use tracing::debug;

use domain::library::LibraryId;
use domain::user::UserId;

use crate::dto::session::PlaybackSessionResponse;
use crate::dto::user::{
    ChangePasswordRequest, CreateUserRequest, LibraryAccessResponse, RoleDto, SetActiveRequest,
    SetLibraryAccessRequest, UpdateProfileRequest, UserResponse,
};
use crate::error::{ApiError, ApiResult};
use crate::extract::{AuthUser, RequireAdmin};
use crate::handlers::{deny_player, log_fail, require_admin_or_self};
use crate::pagination::{PageParams, PageResponse};
use domain::service::{AuthService, DiscoveryService, SessionService, UserService};

use crate::state::AppServices;

pub async fn list<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Query(page): Query<PageParams>,
) -> ApiResult<Json<PageResponse<UserResponse>>> {
    let actor = &principal.user.0;
    let users = state
        .user()
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
    if matches!(req.role, RoleDto::Automation) {
        return Err(ApiError::bad_request(
            "the automation role cannot be assigned to a user",
        ));
    }
    let user = state
        .user()
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
        .session()
        .active_sessions(&principal, page.to_request())
        .await
        .map_err(log_fail(actor, "retrieve active sessions"))?;
    let total = sessions.total;
    let offset = sessions.offset;
    let limit = sessions.limit;
    let now_playing = state
        .discovery()
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
        .user()
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
        .user()
        .get(&principal.user)
        .await
        .map_err(log_fail(actor, "retrieve current user"))?;
    debug!("User [{actor}] successfully retrieved self");
    Ok(Json(UserResponse {
        role: principal.role.into(),
        ..UserResponse::from(user)
    }))
}

pub async fn update_profile<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateProfileRequest>,
) -> ApiResult<Json<UserResponse>> {
    let actor = &principal.user.0;
    let target = UserId(id);
    deny_player(&principal)?;
    require_admin_or_self(&principal, &target)?;
    let user = state
        .user()
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
        .user()
        .delete(&principal, &target)
        .await
        .map_err(log_fail(actor, "delete a user"))?;
    state
        .session()
        .end_all_for_user(&target)
        .await
        .map_err(log_fail(actor, "stop playback for a deleted user"))?;
    debug!("User [{actor}] successfully deleted user [{}]", target.0);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn set_active<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    Json(req): Json<SetActiveRequest>,
) -> ApiResult<Json<UserResponse>> {
    let actor = &principal.user.0;
    let target = UserId(id);
    let user = state
        .user()
        .set_active(&principal, &target, req.active)
        .await
        .map_err(log_fail(actor, "change whether a user is active"))?;
    if !req.active {
        state
            .session()
            .end_all_for_user(&target)
            .await
            .map_err(log_fail(actor, "stop playback for a deactivated user"))?;
    }
    debug!(
        "User [{actor}] successfully set user [{}] active to [{}]",
        target.0, req.active
    );
    Ok(Json(user.into()))
}

pub async fn library_access<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<LibraryAccessResponse>>> {
    let actor = &principal.user.0;
    let target = UserId(id);
    let access = state
        .user()
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
    deny_player(&principal)?;
    require_admin_or_self(&principal, &target)?;
    state
        .user()
        .change_password(
            &principal,
            &target,
            req.current_password.as_deref(),
            &req.new_password,
        )
        .await
        .map_err(log_fail(actor, "change password"))?;
    state
        .auth()
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
        .user()
        .set_library_access(&target, &libraries)
        .await
        .map_err(log_fail(actor, "set library access"))?;
    debug!(
        "User [{actor}] successfully set library access for user [{}]",
        target.0
    );
    Ok(StatusCode::NO_CONTENT)
}

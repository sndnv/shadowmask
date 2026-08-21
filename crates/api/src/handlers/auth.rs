use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use tracing::debug;

use domain::user::{ApiTokenId, DeviceId, UserId};

use crate::dto::auth::{
    ApiTokenResponse, CreateLinkCodeRequest, CreateLinkCodeResponse, DeviceResponse,
    IssuedTokenResponse, LinkCodeResponse, LinkRequest, LoginRequest, LogoutRequest,
    RefreshRequest, TokenPairResponse,
};
use crate::error::ApiResult;
use crate::extract::AuthUser;
use crate::handlers::{deny_player, log_fail, require_admin_or_self};
use domain::service::AuthService;

use crate::state::AppServices;

pub async fn login<S: AppServices>(
    State(state): State<S>,
    Json(req): Json<LoginRequest>,
) -> ApiResult<Json<TokenPairResponse>> {
    let tokens = state
        .auth()
        .login(&req.username, &req.password)
        .await
        .map_err(log_fail(&req.username, "log in"))?;
    debug!("User [{}] successfully logged in", req.username);
    Ok(Json(tokens.into()))
}

pub async fn refresh<S: AppServices>(
    State(state): State<S>,
    Json(req): Json<RefreshRequest>,
) -> ApiResult<Json<TokenPairResponse>> {
    let tokens = state
        .auth()
        .refresh(&req.refresh_token)
        .await
        .map_err(log_fail("anonymous", "refresh access token"))?;
    debug!("User [anonymous] successfully refreshed access token");
    Ok(Json(tokens.into()))
}

pub async fn link<S: AppServices>(
    State(state): State<S>,
    Json(req): Json<LinkRequest>,
) -> ApiResult<Json<IssuedTokenResponse>> {
    let issued = state
        .auth()
        .redeem_link_code(&req.code, req.device.into())
        .await
        .map_err(log_fail("anonymous", "redeem link code"))?;
    debug!("User [anonymous] successfully redeemed link code");
    Ok(Json(issued.into()))
}

pub async fn create_link<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Json(req): Json<CreateLinkCodeRequest>,
) -> ApiResult<Json<CreateLinkCodeResponse>> {
    let actor = &principal.user.0;
    let target = req
        .user_id
        .clone()
        .map(UserId)
        .unwrap_or_else(|| principal.user.clone());
    deny_player(&principal)?;
    require_admin_or_self(&principal, &target)?;
    let link = state
        .auth()
        .create_link_code(&principal, req.user_id.map(UserId), req.ttl_secs)
        .await
        .map_err(log_fail(actor, "create link code"))?;
    debug!("User [{actor}] successfully created a link code");
    Ok(Json(link.into()))
}

pub async fn link_codes<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(user_id): Path<String>,
) -> ApiResult<Json<Vec<LinkCodeResponse>>> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    deny_player(&principal)?;
    require_admin_or_self(&principal, &target)?;
    let codes = state
        .auth()
        .list_link_codes(&target)
        .await
        .map_err(log_fail(actor, "list link codes"))?;
    debug!(
        "User [{actor}] successfully listed {} link codes for user [{}]",
        codes.len(),
        target.0
    );
    Ok(Json(
        codes.into_iter().map(LinkCodeResponse::from).collect(),
    ))
}

pub async fn revoke_link_code<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path((user_id, code)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    deny_player(&principal)?;
    require_admin_or_self(&principal, &target)?;
    state
        .auth()
        .revoke_link_code(&target, &code)
        .await
        .map_err(log_fail(actor, "revoke a link code"))?;
    debug!(
        "User [{actor}] successfully revoked a link code for user [{}]",
        target.0
    );
    Ok(StatusCode::NO_CONTENT)
}

pub async fn logout<S: AppServices>(
    State(state): State<S>,
    Json(req): Json<LogoutRequest>,
) -> ApiResult<StatusCode> {
    state
        .auth()
        .logout(&req.refresh_token)
        .await
        .map_err(log_fail("anonymous", "log out"))?;
    debug!("User [anonymous] successfully logged out a session");
    Ok(StatusCode::NO_CONTENT)
}

pub async fn logout_all<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(user_id): Path<String>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    state
        .auth()
        .logout_all(&target)
        .await
        .map_err(log_fail(actor, "log out all sessions"))?;
    debug!(
        "User [{actor}] successfully logged out all sessions for user [{}]",
        target.0
    );
    Ok(StatusCode::NO_CONTENT)
}

pub async fn devices<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(user_id): Path<String>,
) -> ApiResult<Json<Vec<DeviceResponse>>> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    let devices = state
        .auth()
        .list_devices(&target)
        .await
        .map_err(log_fail(actor, "list devices"))?;
    debug!(
        "User [{actor}] successfully listed {} devices for user [{}]",
        devices.len(),
        target.0
    );
    Ok(Json(
        devices.into_iter().map(DeviceResponse::from).collect(),
    ))
}

pub async fn revoke_device<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path((user_id, device_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    state
        .auth()
        .revoke_device(&target, &DeviceId(device_id))
        .await
        .map_err(log_fail(actor, "revoke a device"))?;
    debug!(
        "User [{actor}] successfully revoked a device for user [{}]",
        target.0
    );
    Ok(StatusCode::NO_CONTENT)
}

pub async fn tokens<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(user_id): Path<String>,
) -> ApiResult<Json<Vec<ApiTokenResponse>>> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    let tokens = state
        .auth()
        .list_api_tokens(&target)
        .await
        .map_err(log_fail(actor, "list API tokens"))?;
    debug!(
        "User [{actor}] successfully listed {} API tokens for user [{}]",
        tokens.len(),
        target.0
    );
    Ok(Json(
        tokens.into_iter().map(ApiTokenResponse::from).collect(),
    ))
}

pub async fn revoke_token<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path((user_id, token_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    state
        .auth()
        .revoke_api_token(&target, &ApiTokenId(token_id))
        .await
        .map_err(log_fail(actor, "revoke an API token"))?;
    debug!(
        "User [{actor}] successfully revoked an API token for user [{}]",
        target.0
    );
    Ok(StatusCode::NO_CONTENT)
}

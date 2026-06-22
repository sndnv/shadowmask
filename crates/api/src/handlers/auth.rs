use axum::Json;
use axum::extract::State;
use tracing::debug;

use crate::dto::auth::{
    AccessTokenResponse, IssuedTokenResponse, LinkRequest, LoginRequest, RefreshRequest,
    TokenPairResponse,
};
use crate::error::ApiResult;
use crate::handlers::log_fail;
use crate::state::AppServices;

pub async fn login<S: AppServices>(
    State(state): State<S>,
    Json(req): Json<LoginRequest>,
) -> ApiResult<Json<TokenPairResponse>> {
    let tokens = state
        .login(&req.username, &req.password)
        .await
        .map_err(log_fail(&req.username, "log in"))?;
    debug!("User [{}] successfully logged in", req.username);
    Ok(Json(tokens.into()))
}

pub async fn refresh<S: AppServices>(
    State(state): State<S>,
    Json(req): Json<RefreshRequest>,
) -> ApiResult<Json<AccessTokenResponse>> {
    let access = state
        .refresh(&req.refresh_token)
        .await
        .map_err(log_fail("anonymous", "refresh access token"))?;
    debug!("User [anonymous] successfully refreshed access token");
    Ok(Json(access.into()))
}

pub async fn link<S: AppServices>(
    State(state): State<S>,
    Json(req): Json<LinkRequest>,
) -> ApiResult<Json<IssuedTokenResponse>> {
    let issued = state
        .redeem_link_code(&req.code, req.device.into())
        .await
        .map_err(log_fail("anonymous", "redeem link code"))?;
    debug!("User [anonymous] successfully redeemed link code");
    Ok(Json(issued.into()))
}

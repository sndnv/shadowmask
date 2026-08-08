use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use tracing::debug;

use domain::session::SessionId;

use crate::dto::session::{
    HeartbeatAckResponse, HeartbeatRequest, RenegotiatedResponse, SeekRequest,
    SessionStartedResponse, StartSessionRequest, UpdateSessionRequest,
};
use crate::error::ApiResult;
use crate::extract::AuthUser;
use crate::handlers::log_fail;
use crate::state::AppServices;

pub async fn start<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Json(req): Json<StartSessionRequest>,
) -> ApiResult<(StatusCode, Json<SessionStartedResponse>)> {
    let actor = &principal.user.0;
    let started = state
        .start(&principal, req.into())
        .await
        .map_err(log_fail(actor, "start a session"))?;
    debug!("User [{actor}] successfully started a session");
    Ok((StatusCode::CREATED, Json(started.into())))
}

pub async fn heartbeat<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
    Json(req): Json<HeartbeatRequest>,
) -> ApiResult<Json<HeartbeatAckResponse>> {
    let actor = &principal.user.0;
    let id = SessionId(id);
    let ack = state
        .heartbeat(&principal, &id, req.position_ms, req.state.into())
        .await
        .map_err(log_fail(actor, "send a heartbeat"))?;
    debug!(
        "User [{actor}] successfully sent a heartbeat for session [{}]",
        id.0
    );
    Ok(Json(ack.into()))
}

pub async fn seek<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
    Json(req): Json<SeekRequest>,
) -> ApiResult<Json<RenegotiatedResponse>> {
    let actor = &principal.user.0;
    let id = SessionId(id);
    let renegotiated = state
        .seek(&principal, &id, req.position_ms)
        .await
        .map_err(log_fail(actor, "seek"))?;
    debug!("User [{actor}] successfully sought session [{}]", id.0);
    Ok(Json(renegotiated.into()))
}

pub async fn update<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateSessionRequest>,
) -> ApiResult<Json<RenegotiatedResponse>> {
    let actor = &principal.user.0;
    let id = SessionId(id);
    let renegotiated = state
        .update(&principal, &id, req.into())
        .await
        .map_err(log_fail(actor, "update a session"))?;
    debug!("User [{actor}] successfully updated session [{}]", id.0);
    Ok(Json(renegotiated.into()))
}

pub async fn end<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let id = SessionId(id);
    state
        .end(&principal, &id)
        .await
        .map_err(log_fail(actor, "end a session"))?;
    debug!("User [{actor}] successfully ended session [{}]", id.0);
    Ok(StatusCode::NO_CONTENT)
}

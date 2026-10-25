use axum::extract::{Path, State};
use axum::http::StatusCode;
use tracing::debug;

use domain::session::{PlaybackState, SegmentContainer, SessionId, SessionUpdate};

use crate::dto::session::{
    HeartbeatAckResponse, HeartbeatRequest, RenegotiatedResponse, SeekRequest,
    SessionStartedResponse, StartSessionRequest, UpdateSessionRequest,
};
use crate::error::ApiResult;
use crate::extract::{AuthUser, Json};
use crate::handlers::{log_fail, to_completion};
use domain::service::SessionService;

use crate::state::AppServices;

pub async fn start<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Json(req): Json<StartSessionRequest>,
) -> ApiResult<(StatusCode, Json<SessionStartedResponse>)> {
    let actor = &principal.user.0;
    let (version, from_ms) = (req.version_id.clone(), req.start_position_ms);
    let started = state
        .session()
        .start(&principal, req.into())
        .await
        .map_err(log_fail(actor, "start a session"))?;
    #[rustfmt::skip]
    debug!("User [{actor}] started session [{}] for version [{version}] from [{from_ms}] ms: mode [{:?}] container [{}] origin [{}] ms", started.session_id.0, started.mode, started.container.map_or("none", SegmentContainer::as_str), started.origin_ms);
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
    let playback = PlaybackState::from(req.state);
    let ack = state
        .session()
        .heartbeat(&principal, &id, req.position_ms, playback)
        .await
        .map_err(log_fail(actor, "send a heartbeat"))?;
    #[rustfmt::skip]
    debug!("User [{actor}] sent a heartbeat for session [{}] at [{}] ms: [{playback:?}]", id.0, req.position_ms);
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
        .session()
        .seek(&principal, &id, req.position_ms)
        .await
        .map_err(log_fail(actor, "seek"))?;
    #[rustfmt::skip]
    debug!("User [{actor}] sought session [{}] to [{}] ms: mode [{:?}] container [{}] origin [{}] ms", id.0, req.position_ms, renegotiated.mode, renegotiated.container.map_or("none", SegmentContainer::as_str), renegotiated.origin_ms);
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
    let update = SessionUpdate::from(req);
    let renegotiated = state
        .session()
        .update(&principal, &id, update.clone())
        .await
        .map_err(log_fail(actor, "update a session"))?;
    #[rustfmt::skip]
    debug!("User [{actor}] updated session [{}] with [{update:?}]: mode [{:?}] container [{}] origin [{}] ms", id.0, renegotiated.mode, renegotiated.container.map_or("none", SegmentContainer::as_str), renegotiated.origin_ms);
    Ok(Json(renegotiated.into()))
}

pub async fn end<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let actor = principal.user.0.clone();
    let id = SessionId(id);
    let session = id.clone();
    let ended = to_completion(async move { state.session().end(&principal, &session).await });
    ended.await.map_err(log_fail(&actor, "end a session"))?;
    debug!("User [{actor}] ended session [{}]", id.0);
    Ok(StatusCode::NO_CONTENT)
}

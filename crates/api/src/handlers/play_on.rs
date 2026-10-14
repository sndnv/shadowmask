use axum::extract::{Path, State};
use axum::http::StatusCode;
use jiff::Timestamp;
use tracing::debug;
use uuid::Uuid;

use domain::catalog::VersionId;
use domain::error::{AuthError, CatalogError, PlayCommandTokenError};
use domain::service::{AuthService, CatalogService};
use domain::session::{PlayCommandClaims, PlayCommandTokens};
use domain::user::{DeviceId, DeviceTarget};

use crate::dto::play_on::{
    DeviceTargetRequest, PlayCommandRequest, PlayCommandResponse, RedeemCommandRequest,
    RedeemCommandResponse, TargetLocationResponse,
};
use crate::error::{ApiError, ApiResult};
use crate::extract::{AuthUser, Json, RequireDevice};
use crate::handlers::log_fail;
use crate::state::{AppServices, PlayOnState};

const COMMAND_TTL_SECS: i64 = 60;

pub async fn report_target<S, T>(
    State(state): State<PlayOnState<S, T>>,
    RequireDevice(principal, device): RequireDevice,
    Json(req): Json<DeviceTargetRequest>,
) -> ApiResult<StatusCode>
where
    S: AppServices,
    T: PlayCommandTokens + Send + Sync + 'static,
{
    let actor = &principal.user.0;
    let target =
        DeviceTarget::new(&req.kind, &req.addresses, req.port, req.app_id, Timestamp::now())
            .map_err(log_fail(actor, "report a play-on target"))?;
    state
        .services
        .auth()
        .report_target(&device, target)
        .await
        .map_err(log_fail(actor, "report a play-on target"))?;
    debug!("User [{actor}] reported device [{}] as a play-on target", device.0);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn clear_target<S, T>(
    State(state): State<PlayOnState<S, T>>,
    RequireDevice(principal, device): RequireDevice,
) -> ApiResult<StatusCode>
where
    S: AppServices,
    T: PlayCommandTokens + Send + Sync + 'static,
{
    let actor = &principal.user.0;
    state
        .services
        .auth()
        .clear_target(&device)
        .await
        .map_err(log_fail(actor, "clear a play-on target"))?;
    debug!("User [{actor}] cleared the play-on target of device [{}]", device.0);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn mint<S, T>(
    State(state): State<PlayOnState<S, T>>,
    AuthUser(principal): AuthUser,
    Path(device_id): Path<String>,
    Json(req): Json<PlayCommandRequest>,
) -> ApiResult<Json<PlayCommandResponse>>
where
    S: AppServices,
    T: PlayCommandTokens + Send + Sync + 'static,
{
    let actor = &principal.user.0;
    let action = "start playback on another device";
    let device = state
        .services
        .auth()
        .owned_device(&principal.user, &DeviceId(device_id))
        .await
        .map_err(log_fail(actor, action))?;
    let target = device.target.ok_or(AuthError::NotATarget).map_err(log_fail(actor, action))?;
    let version = VersionId(req.version_id);
    let detail = state
        .services
        .catalog()
        .version(&principal, &version)
        .await
        .map_err(log_fail(actor, action))
        .map_err(version_error)?;
    if !detail.version.available {
        debug!("User [{actor}] failed to {action}: version [{}] has no file", version.0);
        return Err(ApiError::version_not_found("version has no playable file"));
    }
    let expires_at = Timestamp::from_second(Timestamp::now().as_second() + COMMAND_TTL_SECS)
        .unwrap_or(Timestamp::MAX);
    let token = state
        .tokens
        .create(&PlayCommandClaims {
            user: principal.user.clone(),
            device: device.id.clone(),
            version: version.clone(),
            expires_at,
            nonce: Uuid::new_v4().to_string(),
        })
        .map_err(log_fail(actor, action))?;
    debug!("User [{actor}] sent version [{}] to device [{}]", version.0, device.id.0);
    Ok(Json(PlayCommandResponse {
        token: token.0,
        expires_at: expires_at.to_string(),
        target: TargetLocationResponse::from(&target),
    }))
}

pub async fn redeem<S, T>(
    State(state): State<PlayOnState<S, T>>,
    RequireDevice(principal, device): RequireDevice,
    Json(req): Json<RedeemCommandRequest>,
) -> ApiResult<Json<RedeemCommandResponse>>
where
    S: AppServices,
    T: PlayCommandTokens + Send + Sync + 'static,
{
    let actor = &principal.user.0;
    let claims =
        state.tokens.verify(&req.token).map_err(log_fail(actor, "redeem a play command"))?;
    if claims.device != device || claims.user != principal.user {
        debug!("User [{actor}] failed to redeem a play command for device [{}]", claims.device.0);
        return Err(PlayCommandTokenError::Invalid.into());
    }
    debug!("User [{actor}] redeemed version [{}] on device [{}]", claims.version.0, device.0);
    Ok(Json(RedeemCommandResponse { version_id: claims.version.0 }))
}

fn version_error(err: CatalogError) -> ApiError {
    match err {
        CatalogError::NotFound => ApiError::version_not_found("version not found"),
        other => other.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use domain::error::RepositoryError;

    #[test]
    fn an_unseen_version_is_version_not_found_and_a_broken_catalog_is_internal() {
        let missing = version_error(CatalogError::NotFound).into_response();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        let broken =
            version_error(CatalogError::Repository(RepositoryError::Backend("boom".into())));
        assert_eq!(broken.into_response().status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}

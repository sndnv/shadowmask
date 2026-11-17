use axum::Json;
use axum::extract::Extension;
use domain::user::Role;
use jiff::Timestamp;

use crate::dto::server::{ServerInfoResponse, SoftwareResponse};
use crate::error::ApiResult;
use crate::extract::{AuthUser, RequireAdmin};
use crate::state::{ServerCapabilities, ServerFonts, ServerStarted, ServerTools};

pub async fn info(
    auth: AuthUser,
    capabilities: Option<Extension<ServerCapabilities>>,
    fonts: Option<Extension<ServerFonts>>,
    started: Option<Extension<ServerStarted>>,
) -> ApiResult<Json<ServerInfoResponse>> {
    let capabilities = capabilities.map(|Extension(c)| c.0).unwrap_or_default();
    let fonts = fonts.and_then(|Extension(f)| f.0);
    let info = ServerInfoResponse::current(capabilities, fonts);
    Ok(Json(match started {
        Some(Extension(ServerStarted(at))) if auth.0.role == Role::Admin => {
            info.with_uptime(at, Timestamp::now())
        }
        _ => info,
    }))
}

pub async fn software(
    _admin: RequireAdmin,
    capabilities: Option<Extension<ServerCapabilities>>,
    tools: Option<Extension<ServerTools>>,
) -> ApiResult<Json<SoftwareResponse>> {
    let features = capabilities.map(|Extension(c)| c.0).unwrap_or_default();
    let yt_dlp = tools.and_then(|Extension(t)| t.yt_dlp);
    Ok(Json(SoftwareResponse::current(yt_dlp, features)))
}

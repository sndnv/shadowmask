use axum::Json;
use axum::extract::Extension;

use crate::dto::server::{ServerInfoResponse, SoftwareResponse};
use crate::error::ApiResult;
use crate::extract::{AuthUser, RequireAdmin};
use crate::state::{ServerCapabilities, ServerFonts, ServerTools};

pub async fn info(
    _auth: AuthUser,
    capabilities: Option<Extension<ServerCapabilities>>,
    fonts: Option<Extension<ServerFonts>>,
) -> ApiResult<Json<ServerInfoResponse>> {
    let capabilities = capabilities.map(|Extension(c)| c.0).unwrap_or_default();
    let fonts = fonts.and_then(|Extension(f)| f.0);
    Ok(Json(ServerInfoResponse::current(capabilities, fonts)))
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

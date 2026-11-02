use axum::Json;
use axum::extract::Extension;

use crate::dto::server::ServerInfoResponse;
use crate::error::ApiResult;
use crate::extract::AuthUser;
use crate::state::{ServerCapabilities, ServerFonts};

pub async fn info(
    _auth: AuthUser,
    capabilities: Option<Extension<ServerCapabilities>>,
    fonts: Option<Extension<ServerFonts>>,
) -> ApiResult<Json<ServerInfoResponse>> {
    let capabilities = capabilities.map(|Extension(c)| c.0).unwrap_or_default();
    let fonts = fonts.and_then(|Extension(f)| f.0);
    Ok(Json(ServerInfoResponse::current(capabilities, fonts)))
}

use axum::Json;
use axum::extract::Extension;

use crate::dto::server::ServerInfoResponse;
use crate::error::ApiResult;
use crate::extract::AuthUser;
use crate::state::ServerCapabilities;

pub async fn info(
    _auth: AuthUser,
    capabilities: Option<Extension<ServerCapabilities>>,
) -> ApiResult<Json<ServerInfoResponse>> {
    let capabilities = capabilities.map(|Extension(c)| c.0).unwrap_or_default();
    Ok(Json(ServerInfoResponse::current(capabilities)))
}

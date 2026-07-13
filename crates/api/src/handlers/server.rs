use axum::Json;

use crate::dto::server::ServerInfoResponse;
use crate::error::ApiResult;
use crate::extract::AuthUser;

pub async fn info(_auth: AuthUser) -> ApiResult<Json<ServerInfoResponse>> {
    Ok(Json(ServerInfoResponse::current()))
}

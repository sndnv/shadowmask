use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;

use domain::common::LogLevel;
use domain::diagnostics::{ServerLogCursor, ServerLogFrom, ServerLogRequest, ServerLogStore};

use crate::dto::server::ServerLogResponse;
use crate::error::{ApiError, ApiResult};
use crate::extract::RequireAdmin;
use crate::state::ServerLogState;

const DEFAULT_LIMIT: usize = 200;
const MAX_LIMIT: usize = 1000;

#[derive(Debug, Deserialize)]
pub struct ServerLogQuery {
    pub limit: Option<usize>,
    pub level: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
}

pub async fn read<L: ServerLogStore>(
    State(state): State<ServerLogState<L>>,
    RequireAdmin(_): RequireAdmin,
    Query(query): Query<ServerLogQuery>,
) -> ApiResult<Json<ServerLogResponse>> {
    let request = ServerLogRequest {
        from: from(query.before.as_deref(), query.after.as_deref())?,
        limit: query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT),
        min_level: level(query.level.as_deref())?,
    };
    let page = state.store.read(request).await?;
    Ok(Json(page.into()))
}

fn from(before: Option<&str>, after: Option<&str>) -> ApiResult<ServerLogFrom> {
    match (before, after) {
        (None, None) => Ok(ServerLogFrom::Newest),
        (Some(text), None) => Ok(ServerLogFrom::Before(cursor(text)?)),
        (None, Some(text)) => Ok(ServerLogFrom::After(cursor(text)?)),
        (Some(_), Some(_)) => Err(ApiError::bad_request("before and after cannot be combined")),
    }
}

fn cursor(text: &str) -> ApiResult<ServerLogCursor> {
    ServerLogCursor::parse(text).ok_or_else(|| ApiError::bad_request("invalid server log cursor"))
}

fn level(token: Option<&str>) -> ApiResult<Option<LogLevel>> {
    token
        .map(|token| {
            LogLevel::parse(token).ok_or_else(|| ApiError::bad_request("unknown log level"))
        })
        .transpose()
}

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use tracing::debug;

use domain::job::{JobId, JobLogLevel, JobLogStore};

use crate::dto::job::JobLogResponse;
use crate::error::{ApiError, ApiResult};
use crate::extract::RequireAdmin;
use crate::state::JobLogState;

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    pub tail: Option<usize>,
    pub level: Option<String>,
}

fn line_level(line: &str) -> Option<JobLogLevel> {
    line.split(' ').nth(1).and_then(JobLogLevel::parse)
}

pub async fn read<J: JobLogStore>(
    State(state): State<JobLogState<J>>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    Query(query): Query<LogQuery>,
) -> ApiResult<Json<JobLogResponse>> {
    let job = JobId(id);
    let min_level = match query.level.as_deref() {
        Some(token) => Some(
            JobLogLevel::parse(token).ok_or_else(|| ApiError::bad_request("unknown log level"))?,
        ),
        None => None,
    };
    let mut lines = state.store.read(&job, None).await?;
    if let Some(min) = min_level {
        lines.retain(|line| line_level(line).is_none_or(|level| level >= min));
    }
    if let Some(limit) = query.tail
        && lines.len() > limit
    {
        lines = lines.split_off(lines.len() - limit);
    }
    let count = lines.len();
    debug!("User [{}] read {count} job log lines for [{}]", principal.user.0, job.0);
    Ok(Json(JobLogResponse { lines }))
}

pub async fn wipe<J: JobLogStore>(
    State(state): State<JobLogState<J>>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let job = JobId(id);
    state.store.wipe(&job).await?;
    debug!("User [{}] wiped job logs for [{}]", principal.user.0, job.0);
    Ok(StatusCode::NO_CONTENT)
}

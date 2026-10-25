use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use jiff::Timestamp;
use serde::Deserialize;
use tracing::debug;
use uuid::Uuid;

use domain::diagnostics::{PlayerId, Report, ReportId, ReportSummary};
use domain::error::RepositoryError;
use domain::repository::ReportRepository;
use domain::user::Principal;

use crate::dto::diagnostics::{
    ReportCreatedResponse, ReportResponse, ReportSummaryResponse, SubmitReportRequest,
};
use crate::error::{ApiError, ApiResult};
use crate::extract::{AuthUser, RequireAdmin};
use crate::handlers::log_fail;
use crate::pagination::{PageParams, PageResponse};
use crate::state::ReportState;

const SEND: &str = "send a diagnostics report";

fn failed<'a>(actor: &'a str, action: &'a str) -> impl FnOnce(RepositoryError) -> ApiError + 'a {
    let log = log_fail(actor, action);
    move |err| {
        log(err);
        ApiError::internal()
    }
}

fn player_of(principal: &Principal, sent: Option<&str>) -> Result<PlayerId, &'static str> {
    if let Some(device) = &principal.device {
        return Ok(PlayerId(device.0.clone()));
    }
    match sent.map(str::trim) {
        Some(id) if !id.is_empty() => Ok(PlayerId(id.to_owned())),
        _ => Err("a player id is required"),
    }
}

fn report_from(principal: &Principal, request: SubmitReportRequest) -> ApiResult<Report> {
    let checked = match request.payload.is_object() {
        true => player_of(principal, request.player_id.as_deref()),
        false => Err("the payload is not a JSON object"),
    };
    let player =
        checked.map_err(log_fail(&principal.user.0, SEND)).map_err(ApiError::bad_request)?;
    let note = request.note.map(|note| note.trim().to_owned()).filter(|note| !note.is_empty());
    Ok(Report {
        summary: ReportSummary {
            id: ReportId(Uuid::new_v4().to_string()),
            created_at: Timestamp::now(),
            user: principal.user.clone(),
            player,
            app: request.app,
            platform: request.platform,
            app_version: request.app_version,
            note,
        },
        payload: request.payload.to_string(),
    })
}

fn found(report: Option<Report>) -> ApiResult<Report> {
    report.ok_or_else(|| ApiError::not_found("report not found"))
}

fn deleted(removed: bool) -> ApiResult<()> {
    match removed {
        true => Ok(()),
        false => Err(ApiError::not_found("report not found")),
    }
}

pub async fn submit<R: ReportRepository>(
    State(state): State<ReportState<R>>,
    AuthUser(principal): AuthUser,
    Json(request): Json<SubmitReportRequest>,
) -> ApiResult<(StatusCode, Json<ReportCreatedResponse>)> {
    let actor = &principal.user.0;
    let report = report_from(&principal, request)?;
    let (id, player) = (report.summary.id.0.clone(), report.summary.player.0.clone());
    state.reports.insert(report).await.map_err(failed(actor, SEND))?;
    debug!("User [{actor}] sent diagnostics report [{id}] from player [{player}]");
    Ok((StatusCode::CREATED, Json(ReportCreatedResponse { id })))
}

#[derive(Debug, Deserialize)]
pub struct ReportsParams {
    pub filter: Option<String>,
}

impl ReportsParams {
    fn needle(&self) -> Option<&str> {
        self.filter.as_deref().map(str::trim).filter(|needle| !needle.is_empty())
    }
}

pub async fn list<R: ReportRepository>(
    State(state): State<ReportState<R>>,
    RequireAdmin(principal): RequireAdmin,
    Query(page): Query<PageParams>,
    Query(params): Query<ReportsParams>,
) -> ApiResult<Json<PageResponse<ReportSummaryResponse>>> {
    let actor = &principal.user.0;
    let since = state.kept_since(Timestamp::now());
    let page = state
        .reports
        .list(since, params.needle(), page.to_request())
        .await
        .map_err(failed(actor, "retrieve diagnostics reports"))?;
    let (shown, total) = (page.items.len(), page.total);
    debug!("User [{actor}] retrieved {shown} of {total} diagnostics reports");
    Ok(Json(PageResponse::from_page(page, ReportSummaryResponse::from)))
}

pub async fn read<R: ReportRepository>(
    State(state): State<ReportState<R>>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<Json<ReportResponse>> {
    let actor = &principal.user.0;
    let id = ReportId(id);
    let since = state.kept_since(Timestamp::now());
    let stored = state
        .reports
        .get(&id, since)
        .await
        .map_err(failed(actor, "retrieve diagnostics report"))?;
    let report = found(stored)?;
    debug!("User [{actor}] retrieved diagnostics report [{}]", id.0);
    Ok(Json(report.into()))
}

pub async fn remove<R: ReportRepository>(
    State(state): State<ReportState<R>>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let id = ReportId(id);
    let removed =
        state.reports.delete(&id).await.map_err(failed(actor, "delete diagnostics report"))?;
    deleted(removed)?;
    debug!("User [{actor}] deleted diagnostics report [{}]", id.0);
    Ok(StatusCode::NO_CONTENT)
}

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use tracing::debug;

use domain::catalog::{EpisodeId, MovieId, SeasonId, SeriesId, VersionId};
use domain::job::{JobId, JobQuery};
use domain::library::LibraryId;
use domain::media::SubtitleFileId;

use crate::dto::catalog::{
    CombineRequest, TranscribeRequest, TranslateRequest, UpscaleRequest, VersionResponse,
};
use crate::dto::job::{JobNodeResponse, JobResponse, JobsResponse};
use crate::dto::library::FetchRequest;
use crate::error::{ApiError, ApiResult};
use crate::extract::RequireAdmin;
use crate::handlers::log_fail;
use crate::pagination::{PageParams, PageResponse};
use domain::service::{CatalogService, JobService, LibraryService};

use crate::state::AppServices;

#[derive(Debug, Deserialize)]
pub struct JobsParams {
    pub state: Option<String>,
    pub filter: Option<String>,
}

impl JobsParams {
    fn to_query(&self) -> ApiResult<JobQuery> {
        let active_only = match self.state.as_deref() {
            None | Some("all") => false,
            Some("active") => true,
            Some(other) => {
                return Err(ApiError::bad_request(format!("unknown job state: {other}")));
            }
        };
        Ok(JobQuery {
            search: self.filter.clone(),
            active_only,
        })
    }
}

pub async fn jobs<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Query(page): Query<PageParams>,
    Query(params): Query<JobsParams>,
) -> ApiResult<Json<JobsResponse>> {
    let actor = &principal.user.0;
    let query = params.to_query()?;
    let jobs = state
        .job()
        .jobs(&principal, &query, page.to_request())
        .await
        .map_err(log_fail(actor, "retrieve jobs"))?;
    let (shown, total) = (jobs.page.items.len(), jobs.page.total);
    debug!("User [{actor}] successfully retrieved {shown} of {total} jobs");
    Ok(Json(jobs.into()))
}

pub async fn job_children<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    Query(page): Query<PageParams>,
) -> ApiResult<Json<PageResponse<JobNodeResponse>>> {
    let actor = &principal.user.0;
    let job_id = JobId(id);
    let children = state
        .job()
        .job_descendants(&principal, &job_id, page.to_request())
        .await
        .map_err(log_fail(actor, "retrieve job children"))?;
    let (shown, id) = (children.items.len(), &job_id.0);
    debug!("User [{actor}] successfully retrieved {shown} children of job [{id}]");
    Ok(Json(PageResponse::from_page(children, Into::into)))
}

pub async fn job<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<Json<JobResponse>> {
    let actor = &principal.user.0;
    let job_id = JobId(id);
    let job = state
        .job()
        .job(&principal, &job_id)
        .await
        .map_err(log_fail(actor, "retrieve job"))?
        .ok_or_else(|| ApiError::not_found("job not found"))?;
    debug!("User [{actor}] successfully retrieved job [{}]", job_id.0);
    Ok(Json(job.into()))
}

pub async fn cancel_job<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let job_id = JobId(id);
    state
        .job()
        .cancel_job(&principal, &job_id)
        .await
        .map_err(log_fail(actor, "cancel job"))?;
    debug!("User [{actor}] successfully cancelled job [{}]", job_id.0);
    Ok(StatusCode::ACCEPTED)
}

pub async fn versions<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Query(page): Query<PageParams>,
) -> ApiResult<Json<PageResponse<VersionResponse>>> {
    let actor = &principal.user.0;
    let versions = state
        .catalog()
        .all_versions(&principal, page.to_request())
        .await
        .map_err(log_fail(actor, "retrieve versions"))?;
    debug!(
        "User [{actor}] successfully retrieved {} versions",
        versions.items.len()
    );
    Ok(Json(PageResponse::from_page(versions, |v| {
        VersionResponse::with_path(v, true)
    })))
}

pub async fn delete_version<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let version = VersionId(id);
    state
        .library()
        .delete_version(&principal, &version)
        .await
        .map_err(log_fail(actor, "delete version"))?;
    debug!(
        "User [{actor}] successfully deleted version [{}]",
        version.0
    );
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_movie<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let movie = MovieId(id);
    state
        .library()
        .delete_movie(&principal, &movie)
        .await
        .map_err(log_fail(actor, "delete movie"))?;
    debug!("User [{actor}] successfully deleted movie [{}]", movie.0);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_series<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let series = SeriesId(id);
    state
        .library()
        .delete_series(&principal, &series)
        .await
        .map_err(log_fail(actor, "delete series"))?;
    debug!("User [{actor}] successfully deleted series [{}]", series.0);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_season<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let season = SeasonId(id);
    state
        .library()
        .delete_season(&principal, &season)
        .await
        .map_err(log_fail(actor, "delete season"))?;
    debug!("User [{actor}] successfully deleted season [{}]", season.0);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_episode<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let episode = EpisodeId(id);
    state
        .library()
        .delete_episode(&principal, &episode)
        .await
        .map_err(log_fail(actor, "delete episode"))?;
    debug!(
        "User [{actor}] successfully deleted episode [{}]",
        episode.0
    );
    Ok(StatusCode::NO_CONTENT)
}

pub async fn transcribe_version<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    Json(req): Json<TranscribeRequest>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let version = VersionId(id);
    state
        .library()
        .trigger_transcription(
            &principal,
            &version,
            req.audio_track_index,
            req.source_language,
        )
        .await
        .map_err(log_fail(actor, "trigger transcription"))?;
    debug!(
        "User [{actor}] successfully triggered transcription for version [{}]",
        version.0
    );
    Ok(StatusCode::ACCEPTED)
}

pub async fn translate_version<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    Json(req): Json<TranslateRequest>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let version = VersionId(id);
    state
        .library()
        .trigger_translation(
            &principal,
            &version,
            &SubtitleFileId(req.source_subtitle_id),
            req.target_language,
        )
        .await
        .map_err(log_fail(actor, "trigger translation"))?;
    debug!(
        "User [{actor}] successfully triggered translation for version [{}]",
        version.0
    );
    Ok(StatusCode::ACCEPTED)
}

pub async fn upscale_version<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    Json(req): Json<UpscaleRequest>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    if req.target_height == 0 {
        return Err(ApiError::bad_request("target_height must be positive"));
    }
    let version = VersionId(id);
    state
        .library()
        .trigger_upscale(&principal, &version, req.target_height)
        .await
        .map_err(log_fail(actor, "trigger upscale"))?;
    debug!(
        "User [{actor}] successfully triggered upscale for version [{}]",
        version.0
    );
    Ok(StatusCode::ACCEPTED)
}

pub async fn create_fetch<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Json(req): Json<FetchRequest>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    req.validate().map_err(ApiError::bad_request)?;
    let library = LibraryId(req.library_id.clone());
    state
        .library()
        .create_fetch(&principal, &library, req.into_input())
        .await
        .map_err(log_fail(actor, "trigger content fetch"))?;
    debug!(
        "User [{actor}] successfully triggered content fetch into library [{}]",
        library.0
    );
    Ok(StatusCode::ACCEPTED)
}

pub async fn combine_subtitles<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    Json(req): Json<CombineRequest>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    if req.top_subtitle_id == req.bottom_subtitle_id {
        return Err(ApiError::bad_request(
            "top and bottom subtitles must differ",
        ));
    }
    let version = VersionId(id);
    state
        .library()
        .trigger_combine(
            &principal,
            &version,
            &SubtitleFileId(req.top_subtitle_id),
            &SubtitleFileId(req.bottom_subtitle_id),
        )
        .await
        .map_err(log_fail(actor, "trigger combine"))?;
    debug!(
        "User [{actor}] successfully triggered subtitle combine for version [{}]",
        version.0
    );
    Ok(StatusCode::ACCEPTED)
}

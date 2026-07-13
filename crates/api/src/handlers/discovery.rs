use axum::Json;
use axum::extract::{Path, Query, State};
use serde::Deserialize;
use tracing::debug;

use domain::common::PageRequest;
use domain::discovery::SearchKind;
use domain::session::PlaybackSession;
use domain::user::UserId;

use crate::dto::discovery::{ContinueResponse, HubResponse, SearchResultResponse};
use crate::error::{ApiError, ApiResult};
use crate::extract::AuthUser;
use crate::handlers::{log_fail, require_admin_or_self};
use crate::pagination::PageResponse;
use crate::state::AppServices;

const DEFAULT_LIMIT: u32 = 50;
const MAX_LIMIT: u32 = 200;

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    #[serde(default)]
    pub q: String,
    #[serde(default)]
    pub offset: u32,
    pub limit: Option<u32>,
    pub r#type: Option<String>,
}

pub async fn search<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Query(params): Query<SearchParams>,
) -> ApiResult<Json<PageResponse<SearchResultResponse>>> {
    let actor = &principal.user.0;
    let page = PageRequest {
        offset: params.offset,
        limit: params.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT),
    };
    let types = match &params.r#type {
        Some(value) => vec![
            SearchKind::parse(value)
                .ok_or_else(|| ApiError::bad_request(format!("unknown search type: {value}")))?,
        ],
        None => Vec::new(),
    };
    let results = state
        .search(&principal.user, &params.q, &types, page)
        .await
        .map_err(log_fail(actor, "search"))?;
    debug!(
        "User [{actor}] successfully retrieved {} search results",
        results.items.len()
    );
    Ok(Json(PageResponse::from_page(
        results,
        SearchResultResponse::from,
    )))
}

pub async fn continue_watching<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(user_id): Path<String>,
) -> ApiResult<Json<ContinueResponse>> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;

    let sessions = state
        .active_sessions(
            &principal,
            PageRequest {
                offset: 0,
                limit: MAX_LIMIT,
            },
        )
        .await
        .map_err(log_fail(actor, "retrieve continue data"))?;
    let mine: Vec<PlaybackSession> = sessions
        .items
        .into_iter()
        .filter(|s| s.user == target)
        .collect();
    let now_playing = state
        .now_playing(mine)
        .await
        .map_err(log_fail(actor, "retrieve continue data"))?;
    let in_progress = state
        .continue_watching(&target)
        .await
        .map_err(log_fail(actor, "retrieve continue data"))?;
    let next_episodes = state
        .next_episodes(&target)
        .await
        .map_err(log_fail(actor, "retrieve continue data"))?;
    let next_movies = state
        .next_movies(&target)
        .await
        .map_err(log_fail(actor, "retrieve continue data"))?;

    debug!(
        "User [{actor}] successfully retrieved continue data for user [{}]",
        target.0
    );
    Ok(Json(ContinueResponse {
        now_playing: now_playing.into_iter().map(Into::into).collect(),
        in_progress: in_progress.into_iter().map(Into::into).collect(),
        next_episodes: next_episodes.into_iter().map(Into::into).collect(),
        next_movies: next_movies.into_iter().map(Into::into).collect(),
    }))
}

pub async fn hub<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(user_id): Path<String>,
) -> ApiResult<Json<Vec<HubResponse>>> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    let hubs = state
        .home_hubs(&target)
        .await
        .map_err(log_fail(actor, "retrieve home hubs"))?;
    debug!(
        "User [{actor}] successfully retrieved {} home hubs for user [{}]",
        hubs.len(),
        target.0
    );
    Ok(Json(hubs.into_iter().map(Into::into).collect()))
}

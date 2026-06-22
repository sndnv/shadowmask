use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use tracing::debug;

use domain::catalog::VersionId;
use domain::user::UserId;

use crate::dto::user_library::{
    AddTitleRequest, FavoriteResponse, PlaybackProgressResponse, WatchHistoryResponse,
    WatchlistItemResponse,
};
use crate::error::ApiResult;
use crate::extract::AuthUser;
use crate::handlers::{log_fail, require_admin_or_self};
use crate::pagination::{PageParams, PageResponse};
use crate::state::AppServices;

pub async fn watchlist<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(user_id): Path<String>,
) -> ApiResult<Json<Vec<WatchlistItemResponse>>> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    let items = state
        .watchlist(&target)
        .await
        .map_err(log_fail(actor, "retrieve watchlist"))?;
    debug!(
        "User [{actor}] successfully retrieved {} watchlist items for user [{}]",
        items.len(),
        target.0
    );
    Ok(Json(items.into_iter().map(Into::into).collect()))
}

pub async fn add_to_watchlist<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path((user_id, title_id)): Path<(String, String)>,
    Json(req): Json<AddTitleRequest>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    let title = req.into_title(title_id);
    state
        .add_to_watchlist(&target, &title)
        .await
        .map_err(log_fail(actor, "add to watchlist"))?;
    debug!(
        "User [{actor}] successfully added title [{}] to watchlist for user [{}]",
        title.id(),
        target.0
    );
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_from_watchlist<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path((user_id, title_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    state
        .remove_from_watchlist(&target, &title_id)
        .await
        .map_err(log_fail(actor, "remove from watchlist"))?;
    debug!(
        "User [{actor}] successfully removed title [{title_id}] from watchlist for user [{}]",
        target.0
    );
    Ok(StatusCode::NO_CONTENT)
}

pub async fn favorites<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(user_id): Path<String>,
) -> ApiResult<Json<Vec<FavoriteResponse>>> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    let items = state
        .favorites(&target)
        .await
        .map_err(log_fail(actor, "retrieve favorites"))?;
    debug!(
        "User [{actor}] successfully retrieved {} favorites for user [{}]",
        items.len(),
        target.0
    );
    Ok(Json(items.into_iter().map(Into::into).collect()))
}

pub async fn add_favorite<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path((user_id, title_id)): Path<(String, String)>,
    Json(req): Json<AddTitleRequest>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    let title = req.into_title(title_id);
    state
        .add_favorite(&target, &title)
        .await
        .map_err(log_fail(actor, "add a favorite"))?;
    debug!(
        "User [{actor}] successfully added title [{}] to favorites for user [{}]",
        title.id(),
        target.0
    );
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_favorite<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path((user_id, title_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    state
        .remove_favorite(&target, &title_id)
        .await
        .map_err(log_fail(actor, "remove a favorite"))?;
    debug!(
        "User [{actor}] successfully removed title [{title_id}] from favorites for user [{}]",
        target.0
    );
    Ok(StatusCode::NO_CONTENT)
}

pub async fn history<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(user_id): Path<String>,
    Query(page): Query<PageParams>,
) -> ApiResult<Json<PageResponse<WatchHistoryResponse>>> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    let page = state
        .history(&target, page.to_request())
        .await
        .map_err(log_fail(actor, "retrieve history"))?;
    debug!(
        "User [{actor}] successfully retrieved {} history entries for user [{}]",
        page.items.len(),
        target.0
    );
    Ok(Json(PageResponse::from_page(
        page,
        WatchHistoryResponse::from,
    )))
}

pub async fn progress<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path((user_id, version)): Path<(String, String)>,
) -> ApiResult<Json<Option<PlaybackProgressResponse>>> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    let version = VersionId(version);
    let progress = state
        .progress(&target, &version)
        .await
        .map_err(log_fail(actor, "retrieve progress"))?;
    debug!(
        "User [{actor}] successfully retrieved progress for version [{}] of user [{}]",
        version.0, target.0
    );
    Ok(Json(progress.map(Into::into)))
}

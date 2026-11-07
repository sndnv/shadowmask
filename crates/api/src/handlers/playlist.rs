use axum::extract::{Path, State};
use axum::http::StatusCode;
use tracing::debug;

use domain::playback::{PlaylistId, QueueEntryId};
use domain::service::PlaylistService;
use domain::user::UserId;

use crate::dto::playlist::{
    AddedResponse, PlaylistDetailResponse, PlaylistItemRequest, PlaylistNameRequest,
    PlaylistPositionRequest, PlaylistResponse, PlaylistSummaryResponse, QueueEntryResponse,
    QueuePlaylistRequest,
};
use crate::error::ApiResult;
use crate::extract::{AuthUser, Json};
use crate::handlers::{audience, log_fail, require_admin_or_self};
use crate::state::PlaylistState;

pub async fn playlists<P: PlaylistService + Send + Sync + 'static>(
    State(state): State<PlaylistState<P>>,
    AuthUser(principal): AuthUser,
    Path(user_id): Path<String>,
) -> ApiResult<Json<Vec<PlaylistSummaryResponse>>> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    let playlists = state
        .service
        .playlists(&audience(&principal, &target))
        .await
        .map_err(log_fail(actor, "retrieve playlists"))?;
    debug!("User [{actor}] retrieved [{}] playlists for user [{}]", playlists.len(), target.0);
    Ok(Json(playlists.into_iter().map(Into::into).collect()))
}

pub async fn create<P: PlaylistService + Send + Sync + 'static>(
    State(state): State<PlaylistState<P>>,
    AuthUser(principal): AuthUser,
    Path(user_id): Path<String>,
    Json(req): Json<PlaylistNameRequest>,
) -> ApiResult<(StatusCode, Json<PlaylistResponse>)> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    let playlist = state
        .service
        .create_playlist(&target, &req.name)
        .await
        .map_err(log_fail(actor, "create a playlist"))?;
    debug!("User [{actor}] created playlist [{}] for user [{}]", playlist.id.0, target.0);
    Ok((StatusCode::CREATED, Json(playlist.into())))
}

pub async fn playlist<P: PlaylistService + Send + Sync + 'static>(
    State(state): State<PlaylistState<P>>,
    AuthUser(principal): AuthUser,
    Path((user_id, playlist_id)): Path<(String, String)>,
) -> ApiResult<Json<PlaylistDetailResponse>> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    let detail = state
        .service
        .playlist(&audience(&principal, &target), &PlaylistId(playlist_id))
        .await
        .map_err(log_fail(actor, "retrieve a playlist"))?;
    #[rustfmt::skip]
    debug!("User [{actor}] retrieved playlist [{}] with [{}] items for user [{}]", detail.playlist.id.0, detail.items.len(), target.0);
    Ok(Json(detail.into()))
}

pub async fn rename<P: PlaylistService + Send + Sync + 'static>(
    State(state): State<PlaylistState<P>>,
    AuthUser(principal): AuthUser,
    Path((user_id, playlist_id)): Path<(String, String)>,
    Json(req): Json<PlaylistNameRequest>,
) -> ApiResult<Json<PlaylistResponse>> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    let playlist = state
        .service
        .rename_playlist(&target, &PlaylistId(playlist_id), &req.name)
        .await
        .map_err(log_fail(actor, "rename a playlist"))?;
    debug!("User [{actor}] renamed playlist [{}] for user [{}]", playlist.id.0, target.0);
    Ok(Json(playlist.into()))
}

pub async fn delete<P: PlaylistService + Send + Sync + 'static>(
    State(state): State<PlaylistState<P>>,
    AuthUser(principal): AuthUser,
    Path((user_id, playlist_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    state
        .service
        .delete_playlist(&target, &PlaylistId(playlist_id.clone()))
        .await
        .map_err(log_fail(actor, "delete a playlist"))?;
    debug!("User [{actor}] deleted playlist [{playlist_id}] for user [{}]", target.0);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn add_item<P: PlaylistService + Send + Sync + 'static>(
    State(state): State<PlaylistState<P>>,
    AuthUser(principal): AuthUser,
    Path((user_id, playlist_id)): Path<(String, String)>,
    Json(req): Json<PlaylistItemRequest>,
) -> ApiResult<Json<AddedResponse>> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    let added = state
        .service
        .add_to_playlist(
            &audience(&principal, &target),
            &PlaylistId(playlist_id.clone()),
            &req.into_target(),
        )
        .await
        .map_err(log_fail(actor, "add to a playlist"))?;
    #[rustfmt::skip]
    debug!("User [{actor}] added [{added}] titles to playlist [{playlist_id}] for user [{}]", target.0);
    Ok(Json(AddedResponse { added }))
}

pub async fn remove_item<P: PlaylistService + Send + Sync + 'static>(
    State(state): State<PlaylistState<P>>,
    AuthUser(principal): AuthUser,
    Path((user_id, playlist_id, title_id)): Path<(String, String, String)>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    state
        .service
        .remove_from_playlist(&target, &PlaylistId(playlist_id.clone()), &title_id)
        .await
        .map_err(log_fail(actor, "remove from a playlist"))?;
    #[rustfmt::skip]
    debug!("User [{actor}] removed title [{title_id}] from playlist [{playlist_id}] for user [{}]", target.0);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn move_item<P: PlaylistService + Send + Sync + 'static>(
    State(state): State<PlaylistState<P>>,
    AuthUser(principal): AuthUser,
    Path((user_id, playlist_id, title_id)): Path<(String, String, String)>,
    Json(req): Json<PlaylistPositionRequest>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    state
        .service
        .move_in_playlist(
            &audience(&principal, &target),
            &PlaylistId(playlist_id.clone()),
            &title_id,
            req.position,
        )
        .await
        .map_err(log_fail(actor, "move a playlist item"))?;
    #[rustfmt::skip]
    debug!("User [{actor}] moved title [{title_id}] to [{}] in playlist [{playlist_id}] for user [{}]", req.position, target.0);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn queue<P: PlaylistService + Send + Sync + 'static>(
    State(state): State<PlaylistState<P>>,
    AuthUser(principal): AuthUser,
    Path(user_id): Path<String>,
) -> ApiResult<Json<Vec<QueueEntryResponse>>> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    let queue = state
        .service
        .queue(&audience(&principal, &target))
        .await
        .map_err(log_fail(actor, "retrieve the queue"))?;
    debug!("User [{actor}] retrieved [{}] queue entries for user [{}]", queue.len(), target.0);
    Ok(Json(queue.into_iter().map(Into::into).collect()))
}

pub async fn queue_playlist<P: PlaylistService + Send + Sync + 'static>(
    State(state): State<PlaylistState<P>>,
    AuthUser(principal): AuthUser,
    Path(user_id): Path<String>,
    Json(req): Json<QueuePlaylistRequest>,
) -> ApiResult<Json<AddedResponse>> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    let added = state
        .service
        .queue_playlist(&target, &req.playlist(), req.shuffle)
        .await
        .map_err(log_fail(actor, "add a playlist to the queue"))?;
    #[rustfmt::skip]
    debug!("User [{actor}] queued [{added}] titles from playlist [{}], shuffled: [{}], for user [{}]", req.playlist_id, req.shuffle, target.0);
    Ok(Json(AddedResponse { added }))
}

pub async fn clear_queue<P: PlaylistService + Send + Sync + 'static>(
    State(state): State<PlaylistState<P>>,
    AuthUser(principal): AuthUser,
    Path(user_id): Path<String>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    state.service.clear_queue(&target).await.map_err(log_fail(actor, "clear the queue"))?;
    debug!("User [{actor}] cleared the queue for user [{}]", target.0);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_queued<P: PlaylistService + Send + Sync + 'static>(
    State(state): State<PlaylistState<P>>,
    AuthUser(principal): AuthUser,
    Path((user_id, item_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let target = UserId(user_id);
    require_admin_or_self(&principal, &target)?;
    state
        .service
        .remove_from_queue(&target, &QueueEntryId(item_id.clone()))
        .await
        .map_err(log_fail(actor, "remove from the queue"))?;
    debug!("User [{actor}] removed queue entry [{item_id}] for user [{}]", target.0);
    Ok(StatusCode::NO_CONTENT)
}

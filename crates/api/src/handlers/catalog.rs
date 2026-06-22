use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use tracing::debug;

use domain::catalog::{CollectionId, EpisodeId, MovieId, SeasonId, SeriesId, TitleId};

use crate::dto::catalog::{
    CollectionResponse, CreateCollectionRequest, EpisodeResponse, MovieResponse, SeasonResponse,
    SeriesResponse, UpdateCollectionRequest, VersionResponse,
};
use crate::error::ApiResult;
use crate::extract::{AuthUser, RequireAdmin};
use crate::handlers::log_fail;
use crate::pagination::{PageParams, PageResponse};
use crate::state::AppServices;

pub async fn movies<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Query(page): Query<PageParams>,
) -> ApiResult<Json<PageResponse<MovieResponse>>> {
    let actor = &principal.user.0;
    let page = state
        .movies(page.to_request())
        .await
        .map_err(log_fail(actor, "retrieve movies"))?;
    debug!(
        "User [{actor}] successfully retrieved {} movies",
        page.items.len()
    );
    Ok(Json(PageResponse::from_page(page, MovieResponse::from)))
}

pub async fn movie<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<MovieResponse>> {
    let actor = &principal.user.0;
    let id = MovieId(id);
    let movie = state
        .movie(&id)
        .await
        .map_err(log_fail(actor, "retrieve movie"))?;
    debug!("User [{actor}] successfully retrieved movie [{}]", id.0);
    Ok(Json(movie.into()))
}

pub async fn movie_versions<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<VersionResponse>>> {
    let actor = &principal.user.0;
    let title = TitleId::Movie(MovieId(id));
    let versions = state
        .versions(&title)
        .await
        .map_err(log_fail(actor, "retrieve movie versions"))?;
    debug!(
        "User [{actor}] successfully retrieved {} versions for movie [{}]",
        versions.len(),
        title.id()
    );
    Ok(Json(versions.into_iter().map(Into::into).collect()))
}

pub async fn collections<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Query(page): Query<PageParams>,
) -> ApiResult<Json<PageResponse<CollectionResponse>>> {
    let actor = &principal.user.0;
    let page = state
        .collections(page.to_request())
        .await
        .map_err(log_fail(actor, "retrieve collections"))?;
    debug!(
        "User [{actor}] successfully retrieved {} collections",
        page.items.len()
    );
    Ok(Json(PageResponse::from_page(
        page,
        CollectionResponse::from,
    )))
}

pub async fn collection<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<CollectionResponse>> {
    let actor = &principal.user.0;
    let id = CollectionId(id);
    let collection = state
        .collection(&id)
        .await
        .map_err(log_fail(actor, "retrieve collection"))?;
    debug!(
        "User [{actor}] successfully retrieved collection [{}]",
        id.0
    );
    Ok(Json(collection.into()))
}

pub async fn create_collection<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Json(req): Json<CreateCollectionRequest>,
) -> ApiResult<(StatusCode, Json<CollectionResponse>)> {
    let actor = &principal.user.0;
    let collection = state
        .create_collection(req.into())
        .await
        .map_err(log_fail(actor, "create collection"))?;
    debug!(
        "User [{actor}] successfully created collection [{}]",
        collection.id.0
    );
    Ok((StatusCode::CREATED, Json(collection.into())))
}

pub async fn update_collection<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    Json(req): Json<UpdateCollectionRequest>,
) -> ApiResult<Json<CollectionResponse>> {
    let actor = &principal.user.0;
    let id = CollectionId(id);
    let collection = state
        .update_collection(&id, req.into())
        .await
        .map_err(log_fail(actor, "update collection"))?;
    debug!("User [{actor}] successfully updated collection [{}]", id.0);
    Ok(Json(collection.into()))
}

pub async fn delete_collection<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let id = CollectionId(id);
    state
        .delete_collection(&id)
        .await
        .map_err(log_fail(actor, "delete collection"))?;
    debug!("User [{actor}] successfully deleted collection [{}]", id.0);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn series<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Query(page): Query<PageParams>,
) -> ApiResult<Json<PageResponse<SeriesResponse>>> {
    let actor = &principal.user.0;
    let page = state
        .series(page.to_request())
        .await
        .map_err(log_fail(actor, "retrieve series"))?;
    debug!(
        "User [{actor}] successfully retrieved {} series",
        page.items.len()
    );
    Ok(Json(PageResponse::from_page(page, SeriesResponse::from)))
}

pub async fn series_detail<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<SeriesResponse>> {
    let actor = &principal.user.0;
    let id = SeriesId(id);
    let series = state
        .series_detail(&id)
        .await
        .map_err(log_fail(actor, "retrieve series"))?;
    debug!("User [{actor}] successfully retrieved series [{}]", id.0);
    Ok(Json(series.into()))
}

pub async fn seasons<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(series_id): Path<String>,
) -> ApiResult<Json<Vec<SeasonResponse>>> {
    let actor = &principal.user.0;
    let series_id = SeriesId(series_id);
    let seasons = state
        .seasons(&series_id)
        .await
        .map_err(log_fail(actor, "retrieve seasons"))?;
    debug!(
        "User [{actor}] successfully retrieved {} seasons for series [{}]",
        seasons.len(),
        series_id.0
    );
    Ok(Json(seasons.into_iter().map(Into::into).collect()))
}

pub async fn season<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path((_series_id, season_id)): Path<(String, String)>,
) -> ApiResult<Json<SeasonResponse>> {
    let actor = &principal.user.0;
    let season_id = SeasonId(season_id);
    let season = state
        .season(&season_id)
        .await
        .map_err(log_fail(actor, "retrieve season"))?;
    debug!(
        "User [{actor}] successfully retrieved season [{}]",
        season_id.0
    );
    Ok(Json(season.into()))
}

pub async fn episodes<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path((_series_id, season_id)): Path<(String, String)>,
) -> ApiResult<Json<Vec<EpisodeResponse>>> {
    let actor = &principal.user.0;
    let season_id = SeasonId(season_id);
    let episodes = state
        .episodes(&season_id)
        .await
        .map_err(log_fail(actor, "retrieve episodes"))?;
    debug!(
        "User [{actor}] successfully retrieved {} episodes for season [{}]",
        episodes.len(),
        season_id.0
    );
    Ok(Json(episodes.into_iter().map(Into::into).collect()))
}

pub async fn episode<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path((_series_id, _season_id, episode_id)): Path<(String, String, String)>,
) -> ApiResult<Json<EpisodeResponse>> {
    let actor = &principal.user.0;
    let episode_id = EpisodeId(episode_id);
    let episode = state
        .episode(&episode_id)
        .await
        .map_err(log_fail(actor, "retrieve episode"))?;
    debug!(
        "User [{actor}] successfully retrieved episode [{}]",
        episode_id.0
    );
    Ok(Json(episode.into()))
}

pub async fn episode_versions<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path((_series_id, _season_id, episode_id)): Path<(String, String, String)>,
) -> ApiResult<Json<Vec<VersionResponse>>> {
    let actor = &principal.user.0;
    let title = TitleId::Episode(EpisodeId(episode_id));
    let versions = state
        .versions(&title)
        .await
        .map_err(log_fail(actor, "retrieve episode versions"))?;
    debug!(
        "User [{actor}] successfully retrieved {} versions for episode [{}]",
        versions.len(),
        title.id()
    );
    Ok(Json(versions.into_iter().map(Into::into).collect()))
}

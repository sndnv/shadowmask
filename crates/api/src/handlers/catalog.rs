use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use tracing::debug;

use domain::catalog::{
    CollectionId, EpisodeId, MovieId, SeasonId, SeriesId, SortOrder, TitleId, TitleListQuery,
    TitleRef, TitleSort, VersionId,
};
use domain::library::LibraryId;
use domain::metadata::PersonId;
use domain::user::Role;

use crate::dto::catalog::{
    CollectionResponse, CreateCollectionRequest, EpisodeResponse, GenreDto, MovieDetailResponse,
    MovieResponse, PersonProfileResponse, RefreshRequest, RelinkRequest, SeasonResponse,
    SeriesDetailResponse, SeriesResponse, TitleBatchRequest, TitleCardResponse,
    UpdateCollectionRequest, VersionDetailResponse, VersionResponse,
};
use crate::error::{ApiError, ApiResult};
use crate::extract::{AuthUser, RequireAdmin};
use crate::handlers::log_fail;
use crate::pagination::{PageParams, PageResponse};
use crate::state::AppServices;

const MAX_BATCH: usize = 200;

#[derive(Debug, Deserialize)]
pub struct CatalogListParams {
    pub genres: Option<String>,
    pub library: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

fn parse_genres(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|genre| !genre.is_empty())
        .map(str::to_owned)
        .collect()
}

impl CatalogListParams {
    fn into_query(self) -> ApiResult<TitleListQuery> {
        let sort = match self.sort.as_deref() {
            Some(value) => TitleSort::parse(value)
                .ok_or_else(|| ApiError::bad_request(format!("unknown sort: {value}")))?,
            None => TitleSort::default(),
        };
        let order = match self.order.as_deref() {
            Some(value) => SortOrder::parse(value)
                .ok_or_else(|| ApiError::bad_request(format!("unknown order: {value}")))?,
            None => SortOrder::default(),
        };
        Ok(TitleListQuery {
            genres: self.genres.as_deref().map(parse_genres).unwrap_or_default(),
            library: self.library.map(LibraryId),
            sort,
            order,
        })
    }
}

pub async fn movies<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Query(page): Query<PageParams>,
    Query(params): Query<CatalogListParams>,
) -> ApiResult<Json<PageResponse<MovieResponse>>> {
    let actor = &principal.user.0;
    let query = params.into_query()?;
    let page = state
        .movies(&principal, &query, page.to_request())
        .await
        .map_err(log_fail(actor, "retrieve movies"))?;
    debug!(
        "User [{actor}] successfully retrieved {} movies",
        page.items.len()
    );
    Ok(Json(PageResponse::from_page(page, MovieResponse::from)))
}

pub async fn title_cards<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Json(req): Json<TitleBatchRequest>,
) -> ApiResult<Json<Vec<TitleCardResponse>>> {
    let actor = &principal.user.0;
    if req.titles.len() > MAX_BATCH {
        return Err(ApiError::bad_request(format!(
            "too many titles: {} exceeds maximum of {MAX_BATCH}",
            req.titles.len()
        )));
    }
    let ids: Vec<TitleId> = req.titles.into_iter().map(Into::into).collect();
    let cards = state
        .title_cards(&principal, &ids)
        .await
        .map_err(log_fail(actor, "resolve title cards"))?;
    debug!(
        "User [{actor}] successfully resolved {} title cards",
        cards.len()
    );
    Ok(Json(cards.into_iter().map(Into::into).collect()))
}

pub async fn movie<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<MovieDetailResponse>> {
    let actor = &principal.user.0;
    let id = MovieId(id);
    let movie = state
        .movie(&principal, &id)
        .await
        .map_err(log_fail(actor, "retrieve movie"))?;
    debug!("User [{actor}] successfully retrieved movie [{}]", id.0);
    Ok(Json(movie.into()))
}

pub async fn movie_versions<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
    Query(page): Query<PageParams>,
) -> ApiResult<Json<PageResponse<VersionResponse>>> {
    let actor = &principal.user.0;
    let title = TitleId::Movie(MovieId(id));
    let versions = state
        .versions(&principal, &title, page.to_request())
        .await
        .map_err(log_fail(actor, "retrieve movie versions"))?;
    debug!(
        "User [{actor}] successfully retrieved {} versions for movie [{}]",
        versions.items.len(),
        title.id()
    );
    let include_path = principal.role == Role::Admin;
    Ok(Json(PageResponse::from_page(versions, |v| {
        VersionResponse::with_path(v, include_path)
    })))
}

pub async fn refresh_movie<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    body: Option<Json<RefreshRequest>>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let external_id = body.and_then(|Json(req)| req.external_id).map(Into::into);
    state
        .reidentify(&principal, TitleRef::Movie(MovieId(id)), external_id)
        .await
        .map_err(log_fail(actor, "refresh movie"))?;
    debug!("User [{actor}] successfully queued a metadata refresh for a movie");
    Ok(StatusCode::ACCEPTED)
}

pub async fn refresh_series<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    body: Option<Json<RefreshRequest>>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let external_id = body.and_then(|Json(req)| req.external_id).map(Into::into);
    state
        .reidentify(&principal, TitleRef::Series(SeriesId(id)), external_id)
        .await
        .map_err(log_fail(actor, "refresh series"))?;
    debug!("User [{actor}] successfully queued a metadata refresh for a series");
    Ok(StatusCode::ACCEPTED)
}

pub async fn version_detail<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<VersionDetailResponse>> {
    let actor = &principal.user.0;
    let id = VersionId(id);
    let detail = state
        .version(&principal, &id)
        .await
        .map_err(log_fail(actor, "retrieve version"))?;
    debug!("User [{actor}] successfully retrieved version [{}]", id.0);
    let include_path = principal.role == Role::Admin;
    Ok(Json(VersionDetailResponse::with_path(detail, include_path)))
}

pub async fn relink_version<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    Json(req): Json<RelinkRequest>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let id = VersionId(id);
    state
        .relink_version(&principal, &id, req.target.into())
        .await
        .map_err(log_fail(actor, "relink version"))?;
    debug!(
        "User [{actor}] successfully queued a relink for version [{}]",
        id.0
    );
    Ok(StatusCode::ACCEPTED)
}

pub async fn collections<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Query(page): Query<PageParams>,
) -> ApiResult<Json<PageResponse<CollectionResponse>>> {
    let actor = &principal.user.0;
    let page = state
        .collections(&principal, page.to_request())
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
        .collection(&principal, &id)
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
        .create_collection(&principal, req.into())
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
        .update_collection(&principal, &id, req.into())
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
        .delete_collection(&principal, &id)
        .await
        .map_err(log_fail(actor, "delete collection"))?;
    debug!("User [{actor}] successfully deleted collection [{}]", id.0);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn series<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Query(page): Query<PageParams>,
    Query(params): Query<CatalogListParams>,
) -> ApiResult<Json<PageResponse<SeriesResponse>>> {
    let actor = &principal.user.0;
    let query = params.into_query()?;
    let page = state
        .series(&principal, &query, page.to_request())
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
) -> ApiResult<Json<SeriesDetailResponse>> {
    let actor = &principal.user.0;
    let id = SeriesId(id);
    let series = state
        .series_detail(&principal, &id)
        .await
        .map_err(log_fail(actor, "retrieve series"))?;
    debug!("User [{actor}] successfully retrieved series [{}]", id.0);
    Ok(Json(series.into()))
}

pub async fn person<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<PersonProfileResponse>> {
    let actor = &principal.user.0;
    let id = PersonId(id);
    let profile = state
        .person(&principal, &id)
        .await
        .map_err(log_fail(actor, "retrieve person"))?;
    debug!("User [{actor}] successfully retrieved person [{}]", id.0);
    Ok(Json(profile.into()))
}

pub async fn refresh_person<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let id = PersonId(id);
    state
        .refresh_person(&principal, &id)
        .await
        .map_err(log_fail(actor, "refresh person"))?;
    debug!("User [{actor}] successfully queued a metadata refresh for a person");
    Ok(StatusCode::ACCEPTED)
}

pub async fn genres<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
) -> ApiResult<Json<Vec<GenreDto>>> {
    let actor = &principal.user.0;
    let genres = state
        .genres(&principal)
        .await
        .map_err(log_fail(actor, "retrieve genres"))?;
    debug!(
        "User [{actor}] successfully retrieved {} genres",
        genres.len()
    );
    Ok(Json(genres.into_iter().map(Into::into).collect()))
}

pub async fn seasons<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(series_id): Path<String>,
) -> ApiResult<Json<Vec<SeasonResponse>>> {
    let actor = &principal.user.0;
    let series_id = SeriesId(series_id);
    let seasons = state
        .seasons(&principal, &series_id)
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
        .season(&principal, &season_id)
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
        .episodes(&principal, &season_id)
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
        .episode(&principal, &episode_id)
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
    Query(page): Query<PageParams>,
) -> ApiResult<Json<PageResponse<VersionResponse>>> {
    let actor = &principal.user.0;
    let title = TitleId::Episode(EpisodeId(episode_id));
    let versions = state
        .versions(&principal, &title, page.to_request())
        .await
        .map_err(log_fail(actor, "retrieve episode versions"))?;
    debug!(
        "User [{actor}] successfully retrieved {} versions for episode [{}]",
        versions.items.len(),
        title.id()
    );
    let include_path = principal.role == Role::Admin;
    Ok(Json(PageResponse::from_page(versions, |v| {
        VersionResponse::with_path(v, include_path)
    })))
}

#[cfg(test)]
mod tests {
    use super::parse_genres;

    #[test]
    fn parse_genres_splits_trims_and_drops_blanks() {
        assert!(parse_genres("").is_empty());
        assert!(parse_genres("   ").is_empty());
        assert_eq!(parse_genres("Action"), vec!["Action".to_owned()]);
        assert_eq!(
            parse_genres("Action, Drama"),
            vec!["Action".to_owned(), "Drama".to_owned()]
        );
        assert_eq!(
            parse_genres("Action, ,  Science Fiction ,"),
            vec!["Action".to_owned(), "Science Fiction".to_owned()]
        );
    }
}

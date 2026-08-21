use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use tracing::debug;

use domain::catalog::{
    CollectionId, EpisodeId, MovieId, RandomScope, SeasonId, SeriesId, SortOrder, TitleId,
    TitleKind, TitleListQuery, TitleRef, TitleSort, VersionId,
};
use domain::library::LibraryId;
use domain::metadata::PersonId;
use domain::user::{Principal, Role};

use crate::dto::catalog::{
    CollectionResponse, CreateCollectionRequest, EditEpisodeRequest, EditMovieRequest,
    EditSeriesRequest, EpisodeResponse, GenreDto, MovieDetailResponse, MovieResponse,
    PeopleBatchRequest, PersonProfileResponse, RandomPickResponse, RefreshRequest, RelinkRequest,
    SeasonResponse, SeriesDetailResponse, SeriesResponse, TitleBatchRequest, TitleCardResponse,
    UpdateCollectionRequest, VersionDetailResponse, VersionResponse,
};
use crate::dto::discovery::PersonResponse;
use crate::error::{ApiError, ApiResult};
use crate::extract::{AuthUser, RequireAdmin};
use crate::handlers::log_fail;
use crate::pagination::{PageParams, PageResponse};
use domain::service::{CatalogService, LibraryService};

use crate::state::AppServices;

const MAX_BATCH: usize = 200;

#[derive(Debug, Deserialize)]
pub struct CatalogListParams {
    pub genres: Option<String>,
    pub library: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GenreParams {
    pub kind: Option<String>,
}

impl GenreParams {
    fn kind(&self) -> ApiResult<Option<TitleKind>> {
        match self.kind.as_deref() {
            Some(value) => TitleKind::parse(value)
                .map(Some)
                .ok_or_else(|| ApiError::bad_request(format!("unknown kind: {value}"))),
            None => Ok(None),
        }
    }
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
        .catalog()
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
        .catalog()
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
        .catalog()
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
        .catalog()
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
    let (external_id, force) = match body {
        Some(Json(req)) => (req.external_id.map(Into::into), req.force),
        None => (None, false),
    };
    state
        .library()
        .reidentify(&principal, TitleRef::Movie(MovieId(id)), external_id, force)
        .await
        .map_err(log_fail(actor, "refresh movie"))?;
    debug!("User [{actor}] successfully queued a metadata refresh for a movie force [{force}]");
    Ok(StatusCode::ACCEPTED)
}

pub async fn refresh_series<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    body: Option<Json<RefreshRequest>>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let (external_id, force) = match body {
        Some(Json(req)) => (req.external_id.map(Into::into), req.force),
        None => (None, false),
    };
    state
        .library()
        .reidentify(
            &principal,
            TitleRef::Series(SeriesId(id)),
            external_id,
            force,
        )
        .await
        .map_err(log_fail(actor, "refresh series"))?;
    debug!("User [{actor}] successfully queued a metadata refresh for a series force [{force}]");
    Ok(StatusCode::ACCEPTED)
}

pub async fn edit_movie<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    Json(req): Json<EditMovieRequest>,
) -> ApiResult<Json<MovieResponse>> {
    let actor = &principal.user.0;
    let id = MovieId(id);
    let movie = state
        .library()
        .edit_movie(&principal, &id, req.into())
        .await
        .map_err(log_fail(actor, "edit movie"))?;
    debug!("User [{actor}] successfully edited movie [{}]", id.0);
    Ok(Json(movie.into()))
}

pub async fn edit_series<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    Json(req): Json<EditSeriesRequest>,
) -> ApiResult<Json<SeriesResponse>> {
    let actor = &principal.user.0;
    let id = SeriesId(id);
    let series = state
        .library()
        .edit_series(&principal, &id, req.into())
        .await
        .map_err(log_fail(actor, "edit series"))?;
    debug!("User [{actor}] successfully edited series [{}]", id.0);
    Ok(Json(series.into()))
}

pub async fn edit_episode<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path((_series, _season, id)): Path<(String, String, String)>,
    Json(req): Json<EditEpisodeRequest>,
) -> ApiResult<Json<EpisodeResponse>> {
    let actor = &principal.user.0;
    let id = EpisodeId(id);
    let episode = state
        .library()
        .edit_episode(&principal, &id, req.try_into()?)
        .await
        .map_err(log_fail(actor, "edit episode"))?;
    debug!("User [{actor}] successfully edited episode [{}]", id.0);
    Ok(Json(episode.into()))
}

pub async fn version_detail<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<VersionDetailResponse>> {
    let actor = &principal.user.0;
    let id = VersionId(id);
    let detail = state
        .catalog()
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
        .library()
        .relink_version(&principal, &id, req.target.into())
        .await
        .map_err(log_fail(actor, "relink version"))?;
    debug!(
        "User [{actor}] successfully queued a relink for version [{}]",
        id.0
    );
    Ok(StatusCode::ACCEPTED)
}

pub async fn relink_series<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Path(id): Path<String>,
    Json(req): Json<RelinkRequest>,
) -> ApiResult<StatusCode> {
    let actor = &principal.user.0;
    let id = SeriesId(id);
    state
        .library()
        .relink_series(&principal, &id, req.target.into())
        .await
        .map_err(log_fail(actor, "relink series"))?;
    debug!(
        "User [{actor}] successfully queued a relink for every version of series [{}]",
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
        .catalog()
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
        .catalog()
        .collection(&principal, &id)
        .await
        .map_err(log_fail(actor, "retrieve collection"))?;
    debug!(
        "User [{actor}] successfully retrieved collection [{}]",
        id.0
    );
    Ok(Json(collection.into()))
}

pub async fn people_batch<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Json(req): Json<PeopleBatchRequest>,
) -> ApiResult<Json<Vec<PersonResponse>>> {
    let actor = &principal.user.0;
    if req.people.len() > MAX_BATCH {
        return Err(ApiError::bad_request(format!(
            "too many people: {} exceeds maximum of {MAX_BATCH}",
            req.people.len()
        )));
    }
    let ids: Vec<PersonId> = req.people.into_iter().map(PersonId).collect();
    let people = state
        .catalog()
        .people_cards(&principal, &ids)
        .await
        .map_err(log_fail(actor, "resolve people cards"))?;
    debug!(
        "User [{actor}] successfully resolved {} people cards",
        people.len()
    );
    Ok(Json(people.into_iter().map(PersonResponse::from).collect()))
}

pub async fn movie_collections<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<CollectionResponse>>> {
    let actor = &principal.user.0;
    let movie = MovieId(id);
    let found = state
        .catalog()
        .movie_collections(&principal, &movie)
        .await
        .map_err(log_fail(actor, "retrieve the collections of a movie"))?;
    debug!(
        "User [{actor}] successfully retrieved {} collections of movie [{}]",
        found.len(),
        movie.0
    );
    Ok(Json(
        found.into_iter().map(CollectionResponse::from).collect(),
    ))
}

pub async fn create_collection<S: AppServices>(
    State(state): State<S>,
    RequireAdmin(principal): RequireAdmin,
    Json(req): Json<CreateCollectionRequest>,
) -> ApiResult<(StatusCode, Json<CollectionResponse>)> {
    let actor = &principal.user.0;
    let collection = state
        .catalog()
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
        .catalog()
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
        .catalog()
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
        .catalog()
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
        .catalog()
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
        .catalog()
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
        .library()
        .refresh_person(&principal, &id)
        .await
        .map_err(log_fail(actor, "refresh person"))?;
    debug!("User [{actor}] successfully queued a metadata refresh for a person");
    Ok(StatusCode::ACCEPTED)
}

pub async fn genres<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Query(params): Query<GenreParams>,
) -> ApiResult<Json<Vec<GenreDto>>> {
    let actor = &principal.user.0;
    let kind = params.kind()?;
    let genres = state
        .catalog()
        .genres(&principal, kind)
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
        .catalog()
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
        .catalog()
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
        .catalog()
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
        .catalog()
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
        .catalog()
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

async fn pick<S: AppServices>(
    state: &S,
    principal: &Principal,
    scope: RandomScope,
    query: &TitleListQuery,
    surface: &str,
) -> ApiResult<Json<RandomPickResponse>> {
    let actor = &principal.user.0;
    let action = format!("pick a random title from {surface}");
    let version = state
        .catalog()
        .random(principal, &scope, query)
        .await
        .map_err(log_fail(actor, &action))?;
    debug!(
        "User [{actor}] successfully picked version [{}] at random",
        version.0
    );
    Ok(Json(RandomPickResponse {
        version_id: version.0,
    }))
}

pub async fn random_movie<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Query(params): Query<CatalogListParams>,
) -> ApiResult<Json<RandomPickResponse>> {
    let query = params.into_query()?;
    pick(&state, &principal, RandomScope::Movies, &query, "movies").await
}

pub async fn random_episode<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Query(params): Query<CatalogListParams>,
) -> ApiResult<Json<RandomPickResponse>> {
    let query = params.into_query()?;
    pick(&state, &principal, RandomScope::Episodes, &query, "series").await
}

pub async fn random_in_collection<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<RandomPickResponse>> {
    let scope = RandomScope::Collection(CollectionId(id));
    pick(
        &state,
        &principal,
        scope,
        &TitleListQuery::default(),
        "a collection",
    )
    .await
}

pub async fn random_in_series<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<RandomPickResponse>> {
    let scope = RandomScope::Series(SeriesId(id));
    pick(
        &state,
        &principal,
        scope,
        &TitleListQuery::default(),
        "a series",
    )
    .await
}

pub async fn random_in_season<S: AppServices>(
    State(state): State<S>,
    AuthUser(principal): AuthUser,
    Path((_series_id, season_id)): Path<(String, String)>,
) -> ApiResult<Json<RandomPickResponse>> {
    let scope = RandomScope::Season(SeasonId(season_id));
    pick(
        &state,
        &principal,
        scope,
        &TitleListQuery::default(),
        "a season",
    )
    .await
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

use std::collections::{HashMap, HashSet};

use domain::catalog::{
    ArtworkRef, Collection, CollectionDetail, CollectionId, CollectionUpdate, Episode, EpisodeCard,
    EpisodeContext, EpisodeId, FilmographyEntry, Movie, MovieDetail, MovieId, NewCollection,
    PersonProfile, RandomScope, Season, SeasonCard, SeasonId, Series, SeriesCard, SeriesDetail,
    SeriesId, TitleCard, TitleId, TitleKind, TitleListFilter, TitleListQuery, TitleRef, Version,
    VersionDetail, VersionFilter, VersionId, best_available, in_year_order,
};
use domain::common::{Page, PageRequest};
use domain::error::CatalogError;
use domain::library::LibraryId;
use domain::metadata::{ArtworkKind, Genre, Person, PersonId};
use domain::repository::{CatalogRepository, UserRepository};
use domain::service::CatalogService;
use domain::user::Principal;
use jiff::Timestamp;
use uuid::Uuid;

use crate::acl;
use crate::discovery::{episode_cards, visible_cards, visible_titles};
use crate::page::paginate;

const MOSAIC_TILES: usize = 4;

fn collection_poster(movie: &Movie) -> Option<ArtworkRef> {
    movie.artwork.iter().find(|art| art.kind == ArtworkKind::Poster).cloned()
}

#[derive(Clone)]
pub struct CatalogServiceImpl<C, U> {
    catalog: C,
    users: U,
}

impl<C, U> CatalogServiceImpl<C, U> {
    pub fn new(catalog: C, users: U) -> Self {
        Self { catalog, users }
    }
}

impl<C, U> CatalogServiceImpl<C, U>
where
    C: CatalogRepository + Send + Sync,
    U: UserRepository + Sync,
{
    async fn sorted_members(&self, members: &[MovieId]) -> Result<Vec<MovieId>, CatalogError> {
        let known = self.catalog.movies_by_ids(members).await?;
        Ok(in_year_order(members, &known))
    }

    async fn viewer(&self, caller: &Principal) -> Result<acl::Viewer, CatalogError> {
        Ok(acl::viewer(&self.users, &caller.user).await?.limited_to(caller.age_limit))
    }

    async fn visible_members(
        &self,
        viewer: &acl::Viewer,
        members: &[MovieId],
    ) -> Result<HashMap<MovieId, Movie>, CatalogError> {
        let movies = self.catalog.visible_movies(members, &viewer.filter(None)).await?;
        Ok(movies.into_iter().map(|movie| (movie.id.clone(), movie)).collect())
    }

    async fn visible_collection(
        &self,
        viewer: &acl::Viewer,
        id: &CollectionId,
    ) -> Result<CollectionDetail, CatalogError> {
        let mut collection =
            self.catalog.get_collection(id).await?.ok_or(CatalogError::NotFound)?;
        let mut visible = self.visible_members(viewer, &collection.movies).await?;
        if !collection.movies.is_empty() && visible.is_empty() {
            return Err(CatalogError::NotFound);
        }
        collection.movies.retain(|movie| visible.contains_key(movie));
        let movies: Vec<Movie> =
            collection.movies.iter().filter_map(|movie| visible.remove(movie)).collect();
        if collection.artwork.is_empty() {
            collection.artwork =
                movies.iter().take(MOSAIC_TILES).filter_map(collection_poster).collect();
        }
        Ok(CollectionDetail { collection, movies })
    }

    async fn title_names(
        &self,
        versions: &[Version],
    ) -> Result<HashMap<TitleId, Vec<String>>, CatalogError> {
        let mut movies: Vec<MovieId> = Vec::new();
        let mut episodes: Vec<EpisodeId> = Vec::new();
        for title in versions.iter().map(|version| &version.title).collect::<HashSet<_>>() {
            match title {
                TitleId::Movie(id) => movies.push(id.clone()),
                TitleId::Episode(id) => episodes.push(id.clone()),
            }
        }
        let mut names: HashMap<TitleId, Vec<String>> = HashMap::new();
        for movie in self.catalog.movies_by_ids(&movies).await? {
            names.insert(TitleId::Movie(movie.id), vec![movie.title]);
        }
        let contexts =
            self.catalog.visible_episodes(&episodes, &TitleListFilter::default()).await?;
        let series: Vec<SeriesId> = contexts
            .iter()
            .map(|context| context.series.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let series_titles: HashMap<SeriesId, String> = self
            .catalog
            .series_by_ids(&series)
            .await?
            .into_iter()
            .map(|found| (found.id, found.title))
            .collect();
        for context in contexts {
            let mut titles = vec![context.episode.title];
            titles.extend(series_titles.get(&context.series).cloned());
            names.insert(TitleId::Episode(context.episode.id), titles);
        }
        Ok(names)
    }

    async fn collection_mosaic_art(
        &self,
        movies: &[MovieId],
    ) -> Result<Vec<ArtworkRef>, CatalogError> {
        let tiles = &movies[..movies.len().min(MOSAIC_TILES)];
        Ok(self.catalog.movies_by_ids(tiles).await?.iter().filter_map(collection_poster).collect())
    }

    async fn visible_series_episodes(
        &self,
        viewer: &acl::Viewer,
        series: &[SeriesId],
    ) -> Result<Vec<EpisodeContext>, CatalogError> {
        let episodes: Vec<EpisodeId> =
            self.catalog.episode_ids_for_series(series).await?.into_values().flatten().collect();
        Ok(self.catalog.visible_episodes(&episodes, &viewer.filter(None)).await?)
    }

    async fn build_filter(
        &self,
        caller: &Principal,
        query: &TitleListQuery,
    ) -> Result<TitleListFilter, CatalogError> {
        let mut filter = self.viewer(caller).await?.filter(query.library.as_ref());
        filter.genres = query.genres.clone();
        filter.sort = query.sort;
        filter.order = query.order;
        Ok(filter)
    }
}

impl<C, U> CatalogService for CatalogServiceImpl<C, U>
where
    C: CatalogRepository + Send + Sync,
    U: UserRepository + Sync,
{
    async fn collections(
        &self,
        caller: &Principal,
        page: PageRequest,
    ) -> Result<Page<Collection>, CatalogError> {
        let viewer = self.viewer(caller).await?;
        let all = self.catalog.list_collections(PageRequest::ALL).await?.items;
        let members: Vec<MovieId> = all.iter().flat_map(|c| c.movies.iter().cloned()).collect();
        let visible = self.visible_members(&viewer, &members).await?;
        let shown: Vec<Collection> = all
            .into_iter()
            .filter_map(|mut collection| {
                let had_members = !collection.movies.is_empty();
                collection.movies.retain(|movie| visible.contains_key(movie));
                (!had_members || !collection.movies.is_empty()).then_some(collection)
            })
            .collect();
        let mut page = paginate(&shown, page);
        for collection in page.items.iter_mut() {
            if collection.artwork.is_empty() {
                collection.artwork = self.collection_mosaic_art(&collection.movies).await?;
            }
        }
        Ok(page)
    }

    async fn collection(
        &self,
        caller: &Principal,
        id: &CollectionId,
    ) -> Result<CollectionDetail, CatalogError> {
        self.visible_collection(&self.viewer(caller).await?, id).await
    }

    async fn movie_collections(
        &self,
        caller: &Principal,
        movie: &MovieId,
    ) -> Result<Vec<CollectionDetail>, CatalogError> {
        let viewer = self.viewer(caller).await?;
        if self.visible_members(&viewer, std::slice::from_ref(movie)).await?.is_empty() {
            return Ok(Vec::new());
        }
        let ids = self.catalog.collections_of_movie(movie).await?;
        let mut found = Vec::with_capacity(ids.len());
        for id in &ids {
            found.push(self.visible_collection(&viewer, id).await?);
        }
        found.sort_by(|a, b| {
            a.collection
                .name
                .to_lowercase()
                .cmp(&b.collection.name.to_lowercase())
                .then_with(|| a.collection.id.0.cmp(&b.collection.id.0))
        });
        Ok(found)
    }

    async fn create_collection(
        &self,
        _caller: &Principal,
        input: NewCollection,
    ) -> Result<Collection, CatalogError> {
        let now = Timestamp::now();
        let collection = Collection {
            id: CollectionId(Uuid::new_v4().to_string()),
            name: input.name,
            overview: input.overview,
            movies: self.sorted_members(&input.movies).await?,
            added_at: now,
            updated_at: now,
            artwork: Vec::new(),
        };
        self.catalog.upsert_collection(collection.clone()).await?;
        Ok(collection)
    }

    async fn update_collection(
        &self,
        caller: &Principal,
        id: &CollectionId,
        update: CollectionUpdate,
    ) -> Result<Collection, CatalogError> {
        let existing = self.catalog.get_collection(id).await?.ok_or(CatalogError::NotFound)?;
        let visible = self.visible_members(&self.viewer(caller).await?, &existing.movies).await?;
        let hidden: Vec<MovieId> = existing
            .movies
            .into_iter()
            .filter(|movie| !visible.contains_key(movie) && !update.movies.contains(movie))
            .collect();
        let members: Vec<MovieId> = update.movies.into_iter().chain(hidden).collect();
        let now = Timestamp::now();
        let collection = Collection {
            id: id.clone(),
            name: update.name,
            overview: update.overview,
            movies: self.sorted_members(&members).await?,
            added_at: now,
            updated_at: now,
            artwork: Vec::new(),
        };
        self.catalog.upsert_collection(collection.clone()).await?;
        Ok(collection)
    }

    async fn delete_collection(
        &self,
        _caller: &Principal,
        id: &CollectionId,
    ) -> Result<(), CatalogError> {
        self.catalog.get_collection(id).await?.ok_or(CatalogError::NotFound)?;
        self.catalog.delete_collection(id).await?;
        Ok(())
    }

    async fn movies(
        &self,
        caller: &Principal,
        query: &TitleListQuery,
        page: PageRequest,
    ) -> Result<Page<Movie>, CatalogError> {
        let filter = self.build_filter(caller, query).await?;
        Ok(self.catalog.list_movies_filtered(&filter, page).await?)
    }

    async fn movie(&self, caller: &Principal, id: &MovieId) -> Result<MovieDetail, CatalogError> {
        let viewer = self.viewer(caller).await?;
        if self.visible_members(&viewer, std::slice::from_ref(id)).await?.is_empty() {
            return Err(CatalogError::NotFound);
        }
        self.catalog.movie_detail(id).await?.ok_or(CatalogError::NotFound)
    }

    async fn series(
        &self,
        caller: &Principal,
        query: &TitleListQuery,
        page: PageRequest,
    ) -> Result<Page<SeriesCard>, CatalogError> {
        let filter = self.build_filter(caller, query).await?;
        let Page { items, total, offset, limit } =
            self.catalog.list_series_filtered(&filter, page).await?;
        let ids: Vec<SeriesId> = items.iter().map(|show| show.id.clone()).collect();
        let counts = self.catalog.season_counts(&ids).await?;
        let items = items.into_iter().map(|show| SeriesCard::counted(show, &counts)).collect();
        Ok(Page { items, total, offset, limit })
    }

    async fn series_detail(
        &self,
        caller: &Principal,
        id: &SeriesId,
    ) -> Result<SeriesDetail, CatalogError> {
        let viewer = self.viewer(caller).await?;
        if self.visible_series_episodes(&viewer, std::slice::from_ref(id)).await?.is_empty() {
            return Err(CatalogError::NotFound);
        }
        self.catalog.series_detail(id).await?.ok_or(CatalogError::NotFound)
    }

    async fn seasons(
        &self,
        caller: &Principal,
        series: &SeriesId,
    ) -> Result<Vec<Season>, CatalogError> {
        let viewer = self.viewer(caller).await?;
        let shown: HashSet<SeasonId> = self
            .visible_series_episodes(&viewer, std::slice::from_ref(series))
            .await?
            .into_iter()
            .map(|context| context.season)
            .collect();
        if shown.is_empty() {
            return Err(CatalogError::NotFound);
        }
        let mut seasons = self.catalog.list_seasons(series).await?;
        seasons.retain(|season| shown.contains(&season.id));
        Ok(seasons)
    }

    async fn season(&self, caller: &Principal, id: &SeasonId) -> Result<SeasonCard, CatalogError> {
        let viewer = self.viewer(caller).await?;
        let episodes: Vec<EpisodeId> = self
            .catalog
            .episode_ids_for_seasons(std::slice::from_ref(id))
            .await?
            .into_values()
            .flatten()
            .collect();
        if self.catalog.visible_episodes(&episodes, &viewer.filter(None)).await?.is_empty() {
            return Err(CatalogError::NotFound);
        }
        let season = self.catalog.get_season(id).await?.ok_or(CatalogError::NotFound)?;
        let series = self.catalog.get_series(&season.series).await?;
        Ok(SeasonCard {
            season,
            series_title: series.as_ref().map(|s| s.title.clone()),
            series_artwork: series.map(|s| s.artwork).unwrap_or_default(),
        })
    }

    async fn episodes(
        &self,
        caller: &Principal,
        season: &SeasonId,
    ) -> Result<Vec<Episode>, CatalogError> {
        let viewer = self.viewer(caller).await?;
        let mut episodes = self.catalog.list_episodes(season).await?;
        let ids: Vec<EpisodeId> = episodes.iter().map(|episode| episode.id.clone()).collect();
        let shown: HashSet<EpisodeId> = self
            .catalog
            .visible_episodes(&ids, &viewer.filter(None))
            .await?
            .into_iter()
            .map(|context| context.episode.id)
            .collect();
        if shown.is_empty() {
            return Err(CatalogError::NotFound);
        }
        episodes.retain(|episode| shown.contains(&episode.id));
        Ok(episodes)
    }

    async fn episode(
        &self,
        caller: &Principal,
        id: &EpisodeId,
    ) -> Result<EpisodeCard, CatalogError> {
        let filter = self.viewer(caller).await?.filter(None);
        let contexts = self.catalog.visible_episodes(std::slice::from_ref(id), &filter).await?;
        episode_cards(&self.catalog, contexts).await?.pop().ok_or(CatalogError::NotFound)
    }

    async fn versions(
        &self,
        caller: &Principal,
        title: &TitleId,
        page: PageRequest,
    ) -> Result<Page<Version>, CatalogError> {
        let viewer = self.viewer(caller).await?;
        let titles = std::slice::from_ref(title);
        if visible_titles(&self.catalog, titles, &viewer.filter(None)).await?.is_empty() {
            return Err(CatalogError::NotFound);
        }
        let filtered: Vec<Version> = self
            .catalog
            .list_versions(title, PageRequest::ALL)
            .await?
            .items
            .into_iter()
            .filter(|version| viewer.sees_library(&version.library))
            .collect();
        Ok(paginate(&filtered, page))
    }

    async fn library_versions(
        &self,
        caller: &Principal,
        library: &LibraryId,
        page: PageRequest,
    ) -> Result<Page<Version>, CatalogError> {
        if !self.viewer(caller).await?.sees_library(library) {
            return Ok(paginate(&Vec::<Version>::new(), page));
        }
        Ok(self.catalog.list_library_versions(library, page).await?)
    }

    async fn all_versions(
        &self,
        caller: &Principal,
        filter: &VersionFilter,
        page: PageRequest,
    ) -> Result<Page<Version>, CatalogError> {
        if !acl::is_admin(caller) {
            return Err(CatalogError::Forbidden);
        }
        let viewer = self.viewer(caller).await?;
        let visible: Vec<Version> = self
            .catalog
            .list_all_versions(PageRequest::ALL)
            .await?
            .items
            .into_iter()
            .filter(|version| viewer.sees_library(&version.library))
            .collect();
        let names = if filter.searches_titles() {
            self.title_names(&visible).await?
        } else {
            HashMap::new()
        };
        let matched: Vec<Version> = visible
            .into_iter()
            .filter(|version| {
                filter.matches(version, names.get(&version.title).map_or(&[], Vec::as_slice))
            })
            .collect();
        Ok(paginate(&matched, page))
    }

    async fn version(
        &self,
        caller: &Principal,
        id: &VersionId,
    ) -> Result<VersionDetail, CatalogError> {
        let detail = self.catalog.version_detail(id).await?.ok_or(CatalogError::NotFound)?;
        if !self.viewer(caller).await?.sees_library(&detail.version.library) {
            return Err(CatalogError::NotFound);
        }
        Ok(detail)
    }

    async fn watchable_version(
        &self,
        caller: &Principal,
        id: &VersionId,
    ) -> Result<VersionDetail, CatalogError> {
        let detail = self.catalog.version_detail(id).await?.ok_or(CatalogError::NotFound)?;
        let viewer = self.viewer(caller).await?;
        let rating = self.catalog.title_rating(&detail.version.title).await?;
        if !viewer.sees_library(&detail.version.library) || !viewer.permits(rating.as_ref()) {
            return Err(CatalogError::NotFound);
        }
        Ok(detail)
    }

    async fn people_cards(
        &self,
        _caller: &Principal,
        ids: &[PersonId],
    ) -> Result<Vec<Person>, CatalogError> {
        let mut people = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(person) = self.catalog.get_person(id).await? {
                people.push(person);
            }
        }
        Ok(people)
    }

    async fn person(
        &self,
        caller: &Principal,
        id: &PersonId,
    ) -> Result<PersonProfile, CatalogError> {
        let person = self.catalog.get_person(id).await?.ok_or(CatalogError::NotFound)?;
        let viewer = self.viewer(caller).await?;
        let credits = self.catalog.filmography(id).await?;
        let mut movie_ids = Vec::new();
        let mut series_ids = Vec::new();
        for credit in &credits {
            match &credit.title {
                TitleRef::Movie(movie) => movie_ids.push(movie.clone()),
                TitleRef::Series(series) => series_ids.push(series.clone()),
            }
        }
        let movies = self.visible_members(&viewer, &movie_ids).await?;
        let shown: Vec<SeriesId> = self
            .visible_series_episodes(&viewer, &series_ids)
            .await?
            .into_iter()
            .map(|context| context.series)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let series: HashMap<SeriesId, Series> = self
            .catalog
            .series_by_ids(&shown)
            .await?
            .into_iter()
            .map(|show| (show.id.clone(), show))
            .collect();
        let season_counts = self.catalog.season_counts(&shown).await?;

        let mut filmography = Vec::new();
        for credit in credits {
            let resolved = match &credit.title {
                TitleRef::Movie(movie) => {
                    movies.get(movie).map(|m| (&m.title, m.year, None, &m.artwork))
                }
                TitleRef::Series(show) => series.get(show).map(|s| {
                    let seasons = season_counts.get(show).copied().unwrap_or(0);
                    (&s.title, s.year, Some(seasons), &s.artwork)
                }),
            };
            let Some((display_title, year, season_count, artwork)) = resolved else {
                continue;
            };
            let (display_title, artwork) = (display_title.clone(), artwork.clone());
            filmography.push(FilmographyEntry {
                title: credit.title,
                display_title,
                year,
                season_count,
                artwork,
                role: credit.role,
                character: credit.character,
            });
        }
        Ok(PersonProfile { person, filmography })
    }

    async fn genres(
        &self,
        _caller: &Principal,
        kind: Option<TitleKind>,
    ) -> Result<Vec<Genre>, CatalogError> {
        Ok(self.catalog.list_genres(kind).await?)
    }

    async fn title_cards(
        &self,
        caller: &Principal,
        ids: &[TitleId],
    ) -> Result<Vec<TitleCard>, CatalogError> {
        let filter = self.viewer(caller).await?.filter(None);
        let cards = visible_cards(&self.catalog, ids, &filter).await?;
        Ok(ids.iter().filter_map(|id| cards.get(id).cloned()).collect())
    }

    async fn random(
        &self,
        caller: &Principal,
        scope: &RandomScope,
        query: &TitleListQuery,
    ) -> Result<VersionId, CatalogError> {
        let filter = self.build_filter(caller, query).await?;
        let title = self
            .catalog
            .random_playable_title(scope, &filter)
            .await?
            .ok_or(CatalogError::NotFound)?;
        let versions = self.versions(caller, &title, PageRequest::ALL).await?;
        best_available(&versions.items)
            .map(|version| version.id.clone())
            .ok_or(CatalogError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{ArtworkId, ArtworkOwner, ArtworkWidth, SortOrder, TitleSort};
    use domain::common::Quality;
    use domain::metadata::GenreId;
    use domain::metadata::{ContentRating, Credit, CreditRole, TitleEnrichment};
    use domain::user::{Role, User, UserId};
    use jiff::Timestamp;
    use mocks::{MockCatalogRepo, MockUserRepo};

    fn genre(id: &str, name: &str) -> Genre {
        Genre { id: GenreId(id.into()), name: name.into() }
    }

    fn credit(
        person: &str,
        title: TitleRef,
        role: CreditRole,
        character: Option<&str>,
        order: u32,
    ) -> Credit {
        Credit {
            person: PersonId(person.into()),
            title,
            role,
            character: character.map(Into::into),
            order,
        }
    }

    fn admin() -> Principal {
        Principal {
            user: UserId("admin".into()),
            role: Role::Admin,
            account_admin: true,
            device: None,
            age_limit: None,
        }
    }

    fn member() -> Principal {
        Principal {
            user: UserId("u1".into()),
            role: Role::User,
            account_admin: false,
            device: None,
            age_limit: None,
        }
    }

    fn page() -> PageRequest {
        PageRequest { offset: 0, limit: 10 }
    }

    fn query() -> TitleListQuery {
        TitleListQuery::default()
    }

    fn genre_query(genre: &str) -> TitleListQuery {
        TitleListQuery { genres: vec![genre.to_owned()], ..TitleListQuery::default() }
    }

    fn library_query(library: &str) -> TitleListQuery {
        TitleListQuery { library: Some(LibraryId(library.into())), ..TitleListQuery::default() }
    }

    fn rating(code: &str) -> ContentRating {
        ContentRating { system: "MPAA".into(), code: code.into() }
    }

    fn card_ids(cards: &[TitleCard]) -> Vec<&str> {
        cards
            .iter()
            .map(|card| match card {
                TitleCard::Movie(m) => m.id.0.as_str(),
                TitleCard::Episode(e) => e.episode.id.0.as_str(),
            })
            .collect()
    }

    fn movie(id: &str, code: Option<&str>) -> Movie {
        Movie {
            id: MovieId(id.into()),
            title: id.into(),
            sort_title: id.into(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: code.map(rating),
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    fn version(id: &str, lib: &str) -> Version {
        Version {
            id: VersionId(id.into()),
            title: TitleId::Movie(MovieId("m1".into())),
            library: LibraryId(lib.into()),
            quality: Quality::Hd,
            container: "mkv".into(),
            path: format!("/media/{id}.mkv"),
            size_bytes: 1,
            duration_ms: 1000,
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn capped_user() -> User {
        User {
            id: UserId("u1".into()),
            username: "u1".into(),
            password_hash: "hash".into(),
            role: Role::User,
            max_content_rating: Some(rating("PG-13")),
            preferred_audio: Vec::new(),
            preferred_subtitle: Vec::new(),
            concurrent_stream_limit: None,
            bitrate_cap: None,
            active: true,
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    async fn seeded_catalog() -> MockCatalogRepo {
        seeded_catalog_with(Some(ContentRating { system: "US-TV".into(), code: "TV-PG".into() }))
            .await
    }

    async fn seeded_catalog_with(s2_rating: Option<ContentRating>) -> MockCatalogRepo {
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(movie("m1", Some("PG-13")));
        catalog.add_movie(movie("m2", Some("R")));
        catalog.add_series(Series {
            id: SeriesId("s1".into()),
            title: "s1".into(),
            sort_title: "s1".into(),
            year: None,
            overview: None,
            content_rating: Some(ContentRating { system: "US-TV".into(), code: "TV-MA".into() }),
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog.add_series(Series {
            id: SeriesId("s2".into()),
            title: "s2".into(),
            sort_title: "s2".into(),
            year: None,
            overview: None,
            content_rating: s2_rating,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog.add_season(Season {
            id: SeasonId("se1".into()),
            series: SeriesId("s1".into()),
            number: 1,
            title: None,
            overview: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog.add_episode(Episode {
            id: EpisodeId("e1".into()),
            season: SeasonId("se1".into()),
            number: 1,
            title: "e1".into(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog.add_season(Season {
            id: SeasonId("se2".into()),
            series: SeriesId("s2".into()),
            number: 1,
            title: None,
            overview: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog.add_episode(Episode {
            id: EpisodeId("e2".into()),
            season: SeasonId("se2".into()),
            number: 1,
            title: "e2".into(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog.add_version(version("v1", "lib1"));
        catalog.add_version(version("v2", "lib2"));
        catalog.add_version(Version {
            id: VersionId("ev2".into()),
            title: TitleId::Episode(EpisodeId("e2".into())),
            library: LibraryId("lib1".into()),
            quality: Quality::Hd,
            container: "mkv".into(),
            path: "/media/ev2.mkv".into(),
            size_bytes: 1,
            duration_ms: 1000,
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        });

        catalog
            .upsert_person(Person {
                id: PersonId("p1".into()),
                name: "Ada".into(),
                ..Person::default()
            })
            .await
            .unwrap();
        catalog
            .upsert_person(Person {
                id: PersonId("p2".into()),
                name: "Bob".into(),
                ..Person::default()
            })
            .await
            .unwrap();
        let m1 = TitleRef::Movie(MovieId("m1".into()));
        catalog
            .set_title_enrichment(
                &m1,
                &TitleEnrichment {
                    genres: vec![genre("g-action", "Action")],
                    credits: vec![
                        credit("p1", m1.clone(), CreditRole::Actor, Some("Hero"), 0),
                        credit("p2", m1.clone(), CreditRole::Director, None, 1),
                    ],
                    ..TitleEnrichment::default()
                },
            )
            .await
            .unwrap();
        let m2 = TitleRef::Movie(MovieId("m2".into()));
        catalog
            .set_title_enrichment(
                &m2,
                &TitleEnrichment {
                    genres: vec![genre("g-action", "Action"), genre("g-drama", "Drama")],
                    credits: vec![credit("p1", m2.clone(), CreditRole::Actor, None, 0)],
                    ..TitleEnrichment::default()
                },
            )
            .await
            .unwrap();
        let s1 = TitleRef::Series(SeriesId("s1".into()));
        catalog
            .set_title_enrichment(
                &s1,
                &TitleEnrichment {
                    genres: vec![genre("g-action", "Action")],
                    credits: vec![credit("p1", s1.clone(), CreditRole::Actor, Some("Lead"), 0)],
                    ..TitleEnrichment::default()
                },
            )
            .await
            .unwrap();
        // s2 is the series with a file behind it, so it is the one a scoped genre
        // query can return. s1 stays file-less on purpose, which the random tests pin.
        catalog
            .set_title_enrichment(
                &TitleRef::Series(SeriesId("s2".into())),
                &TitleEnrichment {
                    genres: vec![genre("g-action", "Action")],
                    ..TitleEnrichment::default()
                },
            )
            .await
            .unwrap();
        let ghost = TitleRef::Movie(MovieId("ghost".into()));
        catalog
            .set_title_enrichment(
                &ghost,
                &TitleEnrichment {
                    credits: vec![credit("p1", ghost.clone(), CreditRole::Actor, None, 0)],
                    ..TitleEnrichment::default()
                },
            )
            .await
            .unwrap();
        catalog
    }

    async fn seeded() -> CatalogServiceImpl<MockCatalogRepo, MockUserRepo> {
        serving(seeded_catalog().await).await
    }

    async fn serving(
        catalog: MockCatalogRepo,
    ) -> CatalogServiceImpl<MockCatalogRepo, MockUserRepo> {
        let users = MockUserRepo::new();
        users.insert(capped_user());
        users.set_library_access(&UserId("u1".into()), &[LibraryId("lib1".into())]).await.unwrap();
        // Admin carries no implicit access, so it is granted both libraries here
        // the same way a real admin would grant itself.
        users.grant(&UserId("admin".into()), &[LibraryId("lib1".into()), LibraryId("lib2".into())]);
        CatalogServiceImpl::new(catalog, users)
    }

    #[tokio::test]
    async fn a_fully_granted_uncapped_account_sees_everything_reachable() {
        let svc = seeded().await;
        assert_eq!(
            svc.movies(&admin(), &query(), page()).await.unwrap().total,
            1,
            "m2 has no version, so it sits in no library and a scoped list cannot reach it"
        );
        assert!(svc.movie(&admin(), &MovieId("m1".into())).await.is_ok());
        assert!(
            matches!(
                svc.movie(&admin(), &MovieId("m2".into())).await.unwrap_err(),
                CatalogError::NotFound
            ),
            "a read by id follows the same grants as the list"
        );
        assert_eq!(
            svc.series(&admin(), &query(), page()).await.unwrap().total,
            1,
            "s1 has no file, so like m2 it belongs to no library"
        );
        assert!(svc.series_detail(&admin(), &SeriesId("s2".into())).await.is_ok());
        assert!(matches!(
            svc.series_detail(&admin(), &SeriesId("s1".into())).await.unwrap_err(),
            CatalogError::NotFound
        ));
        assert_eq!(
            svc.versions(&admin(), &TitleId::Movie(MovieId("m1".into())), page())
                .await
                .unwrap()
                .total,
            2
        );
        assert_eq!(
            svc.library_versions(&admin(), &LibraryId("lib2".into()), page()).await.unwrap().total,
            1
        );
        assert!(svc.version(&admin(), &VersionId("v2".into())).await.is_ok());
    }

    #[tokio::test]
    async fn an_admin_without_the_grant_is_refused_like_anyone_else() {
        let svc = seeded().await;
        let stranger = Principal {
            user: UserId("nobody".into()),
            role: Role::Admin,
            account_admin: true,
            device: None,
            age_limit: None,
        };

        assert_eq!(svc.movies(&stranger, &query(), page()).await.unwrap().total, 0);
        assert!(matches!(
            svc.versions(&stranger, &TitleId::Movie(MovieId("m1".into())), page())
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));
        assert!(matches!(
            svc.version(&stranger, &VersionId("v1".into())).await.unwrap_err(),
            CatalogError::NotFound
        ));

        let refused = [
            svc.movie(&stranger, &MovieId("m1".into())).await.err(),
            svc.series_detail(&stranger, &SeriesId("s2".into())).await.err(),
            svc.seasons(&stranger, &SeriesId("s2".into())).await.err(),
            svc.season(&stranger, &SeasonId("se2".into())).await.err(),
            svc.episodes(&stranger, &SeasonId("se2".into())).await.err(),
            svc.episode(&stranger, &EpisodeId("e2".into())).await.err(),
        ];
        assert!(refused.iter().all(|error| matches!(error, Some(CatalogError::NotFound))));
        let profile = svc.person(&stranger, &PersonId("p1".into())).await.unwrap();
        assert!(profile.filmography.is_empty());
        let ids = [TitleId::Movie(MovieId("m1".into())), TitleId::Episode(EpisodeId("e2".into()))];
        assert!(svc.title_cards(&stranger, &ids).await.unwrap().is_empty());
    }

    fn series_rated(id: &str, code: &str) -> Series {
        Series {
            id: SeriesId(id.into()),
            title: id.into(),
            sort_title: id.into(),
            year: None,
            overview: None,
            content_rating: Some(ContentRating { system: "US-TV".into(), code: code.into() }),
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    fn episode_numbered(id: &str, season: &str, number: u16) -> Episode {
        Episode {
            id: EpisodeId(id.into()),
            season: SeasonId(season.into()),
            number,
            title: id.into(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    fn episode_file(id: &str, episode: &str, lib: &str) -> Version {
        titled(id, TitleId::Episode(EpisodeId(episode.into())), lib)
    }

    #[tokio::test]
    async fn a_series_split_across_libraries_shows_each_account_only_its_granted_part() {
        let svc = seeded().await;
        svc.catalog.add_season(season_numbered("se3", "s2", 2));
        svc.catalog.add_episode(episode_numbered("e3", "se3", 1));
        svc.catalog.add_episode(episode_numbered("e4", "se2", 2));
        svc.catalog.add_version(episode_file("ev3", "e3", "lib2"));
        svc.catalog.add_version(episode_file("ev4", "e4", "lib2"));
        let season_ids = |seasons: Vec<Season>| {
            seasons.into_iter().map(|season| season.id.0).collect::<Vec<_>>()
        };
        let episode_ids = |episodes: Vec<Episode>| {
            episodes.into_iter().map(|episode| episode.id.0).collect::<Vec<_>>()
        };

        let s2 = SeriesId("s2".into());
        assert_eq!(season_ids(svc.seasons(&member(), &s2).await.unwrap()), ["se2"]);
        assert_eq!(season_ids(svc.seasons(&admin(), &s2).await.unwrap()), ["se2", "se3"]);
        let se2 = SeasonId("se2".into());
        assert_eq!(episode_ids(svc.episodes(&member(), &se2).await.unwrap()), ["e2"]);
        assert_eq!(episode_ids(svc.episodes(&admin(), &se2).await.unwrap()), ["e2", "e4"]);

        let se3 = SeasonId("se3".into());
        assert!(matches!(svc.season(&member(), &se3).await.unwrap_err(), CatalogError::NotFound));
        assert!(matches!(svc.episodes(&member(), &se3).await.unwrap_err(), CatalogError::NotFound));
        assert!(svc.season(&admin(), &se3).await.is_ok());
        for hidden in ["e3", "e4"] {
            let id = EpisodeId(hidden.into());
            assert!(matches!(
                svc.episode(&member(), &id).await.unwrap_err(),
                CatalogError::NotFound
            ));
            assert!(svc.episode(&admin(), &id).await.is_ok());
        }
    }

    #[tokio::test]
    async fn a_filmography_and_title_cards_leave_out_titles_outside_the_grants() {
        let svc = seeded().await;
        svc.catalog.add_movie(movie("m4", Some("G")));
        svc.catalog.add_version(titled("v4", TitleId::Movie(MovieId("m4".into())), "lib2"));
        svc.catalog.add_series(series_rated("s3", "TV-PG"));
        svc.catalog.add_season(season_numbered("se3", "s3", 1));
        svc.catalog.add_episode(episode_numbered("e3", "se3", 1));
        svc.catalog.add_version(episode_file("ev3", "e3", "lib2"));
        for title in
            [TitleRef::Movie(MovieId("m4".into())), TitleRef::Series(SeriesId("s3".into()))]
        {
            svc.catalog
                .set_title_enrichment(
                    &title,
                    &TitleEnrichment {
                        credits: vec![credit("p1", title.clone(), CreditRole::Actor, None, 0)],
                        ..TitleEnrichment::default()
                    },
                )
                .await
                .unwrap();
        }
        let filmography = |profile: PersonProfile| {
            profile.filmography.iter().map(|entry| entry.title.id().to_owned()).collect::<Vec<_>>()
        };
        let ids = [
            TitleId::Movie(MovieId("m1".into())),
            TitleId::Movie(MovieId("m4".into())),
            TitleId::Episode(EpisodeId("e2".into())),
            TitleId::Episode(EpisodeId("e3".into())),
        ];
        let p1 = PersonId("p1".into());

        assert_eq!(filmography(svc.person(&member(), &p1).await.unwrap()), ["m1"]);
        assert_eq!(filmography(svc.person(&admin(), &p1).await.unwrap()), ["m1", "m4", "s3"]);
        assert_eq!(card_ids(&svc.title_cards(&member(), &ids).await.unwrap()), ["m1", "e2"]);
        assert_eq!(
            card_ids(&svc.title_cards(&admin(), &ids).await.unwrap()),
            ["m1", "m4", "e2", "e3"]
        );
    }

    fn filed(id: &str, title: TitleId, lib: &str, quality: Quality, path: &str) -> Version {
        Version {
            id: VersionId(id.into()),
            title,
            library: LibraryId(lib.into()),
            quality,
            container: path.rsplit('.').next().unwrap().into(),
            path: path.into(),
            size_bytes: 1,
            duration_ms: 1000,
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn filterable() -> CatalogServiceImpl<MockCatalogRepo, MockUserRepo> {
        let catalog = MockCatalogRepo::new();
        for (id, title) in [("m-drift", "Drift"), ("m-paper", "Paper Skies")] {
            catalog.add_movie(Movie { title: title.into(), ..movie(id, None) });
        }
        catalog.add_series(Series {
            id: SeriesId("s-bloom".into()),
            title: "Static Bloom".into(),
            sort_title: "static bloom".into(),
            year: None,
            overview: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog.add_season(Season {
            id: SeasonId("se-bloom".into()),
            series: SeriesId("s-bloom".into()),
            number: 1,
            title: None,
            overview: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog.add_episode(Episode {
            id: EpisodeId("e-pilot".into()),
            season: SeasonId("se-bloom".into()),
            number: 1,
            title: "Pilot Light".into(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        let drift = TitleId::Movie(MovieId("m-drift".into()));
        let paper = TitleId::Movie(MovieId("m-paper".into()));
        let pilot = TitleId::Episode(EpisodeId("e-pilot".into()));
        catalog.add_version(filed("a-drift", drift, "lib1", Quality::Hd, "/media/Drift.mkv"));
        catalog.add_version(filed(
            "b-paper",
            paper,
            "lib1",
            Quality::Fhd,
            "/media/Neon Harbor (2018)/Neon Harbor.mkv",
        ));
        catalog.add_version(filed("c-pilot", pilot, "lib2", Quality::Uhd, "/tv/sb/s01e01.mp4"));
        let users = MockUserRepo::new();
        users.grant(&UserId("admin".into()), &[LibraryId("lib1".into()), LibraryId("lib2".into())]);
        users.grant(&UserId("half".into()), &[LibraryId("lib1".into())]);
        CatalogServiceImpl::new(catalog, users)
    }

    fn library_names() -> HashMap<LibraryId, String> {
        HashMap::from([
            (LibraryId("lib1".into()), "Films".into()),
            (LibraryId("lib2".into()), "Shows".into()),
        ])
    }

    fn version_filter(needle: &str) -> VersionFilter {
        VersionFilter::new(needle, None, library_names())
    }

    fn version_ids(found: &Page<Version>) -> Vec<&str> {
        found.items.iter().map(|version| version.id.0.as_str()).collect()
    }

    #[tokio::test]
    async fn the_versions_filter_searches_every_field_a_listing_shows() {
        let svc = filterable();
        for (needle, expected) in [
            ("neon harbor", vec!["b-paper"]),
            ("PAPER", vec!["b-paper"]),
            ("pilot light", vec!["c-pilot"]),
            ("static bloom", vec!["c-pilot"]),
            ("shows", vec!["c-pilot"]),
            ("uhd", vec!["c-pilot"]),
            ("mp4", vec!["c-pilot"]),
            ("films", vec!["a-drift", "b-paper"]),
            ("lantern", vec![]),
        ] {
            let found = svc.all_versions(&admin(), &version_filter(needle), page()).await.unwrap();
            assert_eq!(version_ids(&found), expected, "{needle}");
            assert_eq!(found.total, expected.len() as u64, "{needle}");
        }
    }

    #[tokio::test]
    async fn the_versions_filter_runs_before_the_page_is_cut() {
        let svc = filterable();
        let first = PageRequest { offset: 0, limit: 1 };
        let unfiltered = svc.all_versions(&admin(), &version_filter(""), first).await.unwrap();
        assert_eq!(version_ids(&unfiltered), ["a-drift"]);
        assert_eq!(unfiltered.total, 3);

        let filtered = svc.all_versions(&admin(), &version_filter("static"), first).await.unwrap();
        assert_eq!(version_ids(&filtered), ["c-pilot"], "a match from the last page comes first");
        assert_eq!(filtered.total, 1);
    }

    #[tokio::test]
    async fn the_versions_filter_still_respects_the_grants() {
        let svc = filterable();
        let half = Principal {
            user: UserId("half".into()),
            role: Role::Admin,
            account_admin: true,
            device: None,
            age_limit: None,
        };
        let found = svc.all_versions(&half, &version_filter("shows"), page()).await.unwrap();
        assert_eq!(found.total, 0);
    }

    #[tokio::test]
    async fn the_versions_list_narrows_to_available_or_unavailable_before_the_page_is_cut() {
        let svc = filterable();
        let drift = TitleId::Movie(MovieId("m-drift".into()));
        let gone = filed("d-gone", drift, "lib1", Quality::Hd, "/media/Gone.mkv");
        svc.catalog.add_version(Version { available: false, ..gone });
        let first = PageRequest { offset: 0, limit: 1 };
        for (available, needle, expected, total) in [
            (Some(false), "", "d-gone", 1),
            (Some(true), "", "a-drift", 3),
            (Some(false), "films", "d-gone", 1),
        ] {
            let filter = VersionFilter::new(needle, available, library_names());
            let found = svc.all_versions(&admin(), &filter, first).await.unwrap();
            assert_eq!(version_ids(&found), [expected], "{available:?} {needle}");
            assert_eq!(found.total, total, "{available:?} {needle}");
        }
    }

    #[tokio::test]
    async fn all_versions_is_admin_only_and_still_scoped_to_the_grants() {
        let svc = seeded().await;
        let all = svc.all_versions(&admin(), &VersionFilter::default(), page()).await.unwrap();
        assert_eq!(all.total, 3);
        assert!(matches!(
            svc.all_versions(&member(), &VersionFilter::default(), page()).await.unwrap_err(),
            CatalogError::Forbidden
        ));

        let stranger = Principal {
            user: UserId("nobody".into()),
            role: Role::Admin,
            account_admin: true,
            device: None,
            age_limit: None,
        };
        assert_eq!(
            svc.all_versions(&stranger, &VersionFilter::default(), page()).await.unwrap().total,
            0,
            "an admin listing must not show a version whose detail page would 404"
        );
    }

    fn season_numbered(id: &str, series: &str, number: u16) -> Season {
        Season {
            id: SeasonId(id.into()),
            series: SeriesId(series.into()),
            number,
            title: None,
            overview: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    #[tokio::test]
    async fn a_series_counts_its_regular_seasons_and_never_the_specials() {
        let five = seeded().await;
        for series in ["s1", "s2"] {
            for number in [0, 2, 3, 4, 5] {
                five.catalog.add_season(season_numbered(
                    &format!("{series}-{number}"),
                    series,
                    number,
                ));
            }
        }
        let only_specials = seeded().await;
        for (id, series) in [("se1", "s1"), ("se2", "s2")] {
            only_specials.catalog.upsert_season(season_numbered(id, series, 0)).await.unwrap();
        }

        for (svc, expected) in [(five, 5), (only_specials, 0)] {
            svc.catalog.add_version(episode_file("ev1", "e1", "lib1"));
            let browsed = svc.series(&admin(), &query(), page()).await.unwrap();
            assert_eq!(browsed.items[0].season_count, expected);
            let profile = svc.person(&admin(), &PersonId("p1".into())).await.unwrap();
            assert_eq!(
                profile.filmography.iter().map(|entry| entry.season_count).collect::<Vec<_>>(),
                [None, Some(expected)],
                "a movie has no seasons to count"
            );
        }
    }

    #[tokio::test]
    async fn member_rating_gate_filters_and_hides() {
        let svc = seeded().await;
        svc.catalog.add_version(titled("vm2", TitleId::Movie(MovieId("m2".into())), "lib1"));
        svc.catalog.add_version(episode_file("ev1", "e1", "lib1"));
        let movies = svc.movies(&member(), &query(), page()).await.unwrap();
        assert_eq!(movies.total, 1);
        assert_eq!(movies.items[0].id, MovieId("m1".into()));
        assert!(svc.movie(&member(), &MovieId("m1".into())).await.is_ok());
        assert!(matches!(
            svc.movie(&member(), &MovieId("m2".into())).await.unwrap_err(),
            CatalogError::NotFound
        ));

        let series = svc.series(&member(), &query(), page()).await.unwrap();
        assert_eq!(series.total, 1);
        assert_eq!(series.items[0].series.id, SeriesId("s2".into()));
        assert!(svc.series_detail(&member(), &SeriesId("s2".into())).await.is_ok());
        assert!(matches!(
            svc.series_detail(&member(), &SeriesId("s1".into())).await.unwrap_err(),
            CatalogError::NotFound
        ));
    }

    #[tokio::test]
    async fn list_scoping_and_sort() {
        let svc = seeded().await;

        let admin_lib1 = svc.movies(&admin(), &library_query("lib1"), page()).await.unwrap();
        assert_eq!(admin_lib1.items.iter().map(|m| m.id.0.as_str()).collect::<Vec<_>>(), ["m1"]);
        let admin_series_lib1 = svc.series(&admin(), &library_query("lib1"), page()).await.unwrap();
        assert_eq!(
            admin_series_lib1.items.iter().map(|s| s.series.id.0.as_str()).collect::<Vec<_>>(),
            ["s2"]
        );
        assert!(
            svc.movies(&admin(), &library_query("ghost"), page()).await.unwrap().items.is_empty()
        );

        assert!(
            svc.movies(&member(), &library_query("lib2"), page()).await.unwrap().items.is_empty()
        );
        let member_lib1 = svc.movies(&member(), &library_query("lib1"), page()).await.unwrap();
        assert_eq!(member_lib1.items.iter().map(|m| m.id.0.as_str()).collect::<Vec<_>>(), ["m1"]);
        let member_series_lib1 =
            svc.series(&member(), &library_query("lib1"), page()).await.unwrap();
        assert_eq!(
            member_series_lib1.items.iter().map(|s| s.series.id.0.as_str()).collect::<Vec<_>>(),
            ["s2"]
        );

        let by_title_desc = TitleListQuery {
            sort: TitleSort::Title,
            order: SortOrder::Desc,
            ..TitleListQuery::default()
        };
        let sorted = svc.movies(&admin(), &by_title_desc, page()).await.unwrap();
        assert_eq!(
            sorted.items.iter().map(|m| m.id.0.as_str()).collect::<Vec<_>>(),
            ["m1"],
            "m2 has no version and so belongs to no library, which every list is scoped by"
        );
    }

    #[tokio::test]
    async fn member_library_gate_filters_versions() {
        let svc = seeded().await;
        let title = TitleId::Movie(MovieId("m1".into()));
        let versions = svc.versions(&member(), &title, page()).await.unwrap();
        assert_eq!(versions.total, 1);
        assert_eq!(versions.items[0].id, VersionId("v1".into()));

        assert_eq!(
            svc.library_versions(&member(), &LibraryId("lib1".into()), page()).await.unwrap().total,
            2
        );
        assert_eq!(
            svc.library_versions(&member(), &LibraryId("lib2".into()), page()).await.unwrap().total,
            0
        );

        assert!(svc.version(&member(), &VersionId("v1".into())).await.is_ok());
        assert!(matches!(
            svc.version(&member(), &VersionId("v2".into())).await.unwrap_err(),
            CatalogError::NotFound
        ));
    }

    #[tokio::test]
    async fn seasons_episodes_and_missing() {
        let svc = seeded().await;
        assert_eq!(svc.seasons(&member(), &SeriesId("s2".into())).await.unwrap().len(), 1);
        assert!(svc.season(&member(), &SeasonId("se2".into())).await.is_ok());
        assert!(matches!(
            svc.season(&member(), &SeasonId("x".into())).await.unwrap_err(),
            CatalogError::NotFound
        ));
        assert_eq!(svc.episodes(&member(), &SeasonId("se2".into())).await.unwrap().len(), 1);
        assert!(
            matches!(
                svc.episodes(&member(), &SeasonId("x".into())).await.unwrap_err(),
                CatalogError::NotFound
            ),
            "a season that is not there holds no episode the account may see"
        );
        assert!(matches!(
            svc.episodes(&admin(), &SeasonId("x".into())).await.unwrap_err(),
            CatalogError::NotFound
        ));
        assert!(svc.episode(&member(), &EpisodeId("e2".into())).await.is_ok());
        assert!(matches!(
            svc.episode(&member(), &EpisodeId("x".into())).await.unwrap_err(),
            CatalogError::NotFound
        ));
        assert!(matches!(
            svc.movie(&member(), &MovieId("x".into())).await.unwrap_err(),
            CatalogError::NotFound
        ));
        assert!(matches!(
            svc.version(&member(), &VersionId("x".into())).await.unwrap_err(),
            CatalogError::NotFound
        ));
    }

    #[tokio::test]
    async fn an_admin_saved_collection_is_stored_in_release_order() {
        let catalog = MockCatalogRepo::new();
        for (id, year) in [("late", 2003u16), ("early", 1999)] {
            let mut m = movie(id, None);
            m.year = Some(year);
            catalog.add_movie(m);
        }
        let svc = CatalogServiceImpl::new(catalog, MockUserRepo::new());
        let members = vec![MovieId("late".into()), MovieId("early".into())];

        let created = svc
            .create_collection(
                &admin(),
                NewCollection { name: "Saga".into(), overview: None, movies: members.clone() },
            )
            .await
            .unwrap();
        assert_eq!(created.movies, vec![MovieId("early".into()), MovieId("late".into())]);

        let updated = svc
            .update_collection(
                &admin(),
                &created.id,
                CollectionUpdate { name: "Saga".into(), overview: None, movies: members },
            )
            .await
            .unwrap();
        assert_eq!(updated.movies, vec![MovieId("early".into()), MovieId("late".into())]);
    }

    #[tokio::test]
    async fn collection_create_update_delete() {
        let svc = seeded().await;
        let created = svc
            .create_collection(
                &admin(),
                NewCollection {
                    name: "Saga".into(),
                    overview: Some("epic".into()),
                    movies: vec![MovieId("m1".into())],
                },
            )
            .await
            .unwrap();
        assert!(Uuid::parse_str(&created.id.0).is_ok());
        assert_eq!(svc.collections(&admin(), page()).await.unwrap().total, 1);
        assert!(svc.collection(&admin(), &created.id).await.is_ok());
        assert!(matches!(
            svc.collection(&admin(), &CollectionId("x".into())).await.unwrap_err(),
            CatalogError::NotFound
        ));

        let updated = svc
            .update_collection(
                &admin(),
                &created.id,
                CollectionUpdate { name: "Saga II".into(), overview: None, movies: Vec::new() },
            )
            .await
            .unwrap();
        assert_eq!(updated.name, "Saga II");
        assert!(matches!(
            svc.update_collection(
                &admin(),
                &CollectionId("x".into()),
                CollectionUpdate { name: "x".into(), overview: None, movies: Vec::new() },
            )
            .await
            .unwrap_err(),
            CatalogError::NotFound
        ));

        svc.delete_collection(&admin(), &created.id).await.unwrap();
        assert!(matches!(
            svc.delete_collection(&admin(), &created.id).await.unwrap_err(),
            CatalogError::NotFound
        ));
    }

    // A method missing from this list has a `?` on the repository whose error arm
    // no test has ever taken, so nothing proves it propagates rather than being
    // swallowed into an empty page or a NotFound.
    #[tokio::test]
    async fn backend_errors_propagate() {
        let catalog = MockCatalogRepo::new();
        catalog.set_fail();
        let users = MockUserRepo::new();
        // Granted, or the library-scoped methods short circuit to an empty page
        // and never reach the repository whose failure this test is about.
        users.grant(&UserId("admin".into()), &[LibraryId("lib1".into()), LibraryId("lib2".into())]);
        let svc = CatalogServiceImpl::new(catalog, users);

        let movie = MovieId("m1".into());
        let series = SeriesId("s1".into());
        let season = SeasonId("se1".into());
        let episode = EpisodeId("e1".into());
        let version = VersionId("v1".into());
        let collection = CollectionId("c1".into());
        let person = PersonId("p1".into());
        let library = LibraryId("lib1".into());
        let title = TitleId::Movie(movie.clone());

        macro_rules! is_repository_error {
            ($call:expr) => {
                assert!(matches!($call.await.unwrap_err(), CatalogError::Repository(_)))
            };
        }

        is_repository_error!(svc.collections(&admin(), page()));
        is_repository_error!(svc.collection(&admin(), &collection));
        is_repository_error!(svc.movie_collections(&admin(), &movie));
        is_repository_error!(svc.create_collection(
            &admin(),
            NewCollection { name: "Saga".into(), overview: None, movies: vec![movie.clone()] }
        ));
        is_repository_error!(svc.update_collection(
            &admin(),
            &collection,
            CollectionUpdate { name: "Saga".into(), overview: None, movies: vec![movie.clone()] }
        ));
        is_repository_error!(svc.delete_collection(&admin(), &collection));
        is_repository_error!(svc.movies(&admin(), &query(), page()));
        is_repository_error!(svc.movie(&admin(), &movie));
        is_repository_error!(svc.series(&admin(), &query(), page()));
        is_repository_error!(svc.series_detail(&admin(), &series));
        is_repository_error!(svc.seasons(&admin(), &series));
        is_repository_error!(svc.season(&admin(), &season));
        is_repository_error!(svc.episodes(&admin(), &season));
        is_repository_error!(svc.episode(&admin(), &episode));
        is_repository_error!(svc.versions(&admin(), &title, page()));
        is_repository_error!(svc.library_versions(&admin(), &library, page()));
        is_repository_error!(svc.all_versions(&admin(), &VersionFilter::default(), page()));
        is_repository_error!(svc.version(&admin(), &version));
        is_repository_error!(svc.watchable_version(&admin(), &version));
        is_repository_error!(svc.people_cards(&admin(), std::slice::from_ref(&person)));
        is_repository_error!(svc.person(&admin(), &person));
        is_repository_error!(svc.genres(&admin(), None));
        is_repository_error!(svc.title_cards(&admin(), &[title]));
        is_repository_error!(svc.random(&admin(), &RandomScope::Movies, &query()));
    }

    // Every gated method builds a Viewer first, and that lookup can fail on its own.
    // Without this the `?` inside `viewer()` is only ever taken on the happy path.
    #[tokio::test]
    async fn an_unreadable_account_fails_the_request_rather_than_hiding_everything() {
        let users = MockUserRepo::new();
        users.set_fail();
        let svc = CatalogServiceImpl::new(MockCatalogRepo::new(), users);

        let movie = MovieId("m1".into());
        let series = SeriesId("s1".into());
        let season = SeasonId("se1".into());
        let episode = EpisodeId("e1".into());
        let library = LibraryId("lib1".into());
        let title = TitleId::Movie(movie.clone());

        macro_rules! is_repository_error {
            ($call:expr) => {
                assert!(matches!($call.await.unwrap_err(), CatalogError::Repository(_)))
            };
        }

        is_repository_error!(svc.movies(&admin(), &query(), page()));
        is_repository_error!(svc.series(&admin(), &query(), page()));
        is_repository_error!(svc.seasons(&admin(), &series));
        is_repository_error!(svc.episodes(&admin(), &season));
        is_repository_error!(svc.versions(&admin(), &title, page()));
        is_repository_error!(svc.library_versions(&admin(), &library, page()));
        is_repository_error!(svc.all_versions(&admin(), &VersionFilter::default(), page()));
        is_repository_error!(svc.random(&admin(), &RandomScope::Movies, &query()));

        let users = MockUserRepo::new();
        users.set_fail();
        let svc = CatalogServiceImpl::new(seeded_catalog().await, users);

        is_repository_error!(svc.movie(&admin(), &movie));
        is_repository_error!(svc.series_detail(&admin(), &series));
        is_repository_error!(svc.season(&admin(), &season));
        is_repository_error!(svc.episode(&admin(), &episode));
        is_repository_error!(svc.person(&admin(), &PersonId("p1".into())));
        is_repository_error!(svc.title_cards(&admin(), &[title]));
    }

    #[tokio::test]
    async fn a_season_and_an_episode_carry_the_context_their_crumbs_need() {
        let catalog = MockCatalogRepo::new();
        catalog.add_series(Series {
            id: SeriesId("s9".into()),
            title: "The Show".into(),
            sort_title: "show, the".into(),
            year: None,
            overview: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog
            .set_artwork(
                &ArtworkOwner::Series(SeriesId("s9".into())),
                &[ArtworkRef {
                    id: ArtworkId("s9-backdrop".into()),
                    kind: ArtworkKind::Backdrop,
                    widths: vec![ArtworkWidth::new(960, "/art/s9-backdrop/960.jpg")],
                }],
            )
            .await
            .unwrap();
        catalog.add_season(Season {
            id: SeasonId("se9".into()),
            series: SeriesId("s9".into()),
            number: 0,
            title: Some("Specials".into()),
            overview: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog.add_episode(Episode {
            id: EpisodeId("e9".into()),
            season: SeasonId("se9".into()),
            number: 3,
            title: "A Christmas Special".into(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog.add_version(episode_file("ev9", "e9", "lib1"));
        let users = MockUserRepo::new();
        users.grant(&UserId("u1".into()), &[LibraryId("lib1".into())]);
        let svc = CatalogServiceImpl::new(catalog, users);

        let season = svc.season(&member(), &SeasonId("se9".into())).await.unwrap();
        assert_eq!(season.series_title.as_deref(), Some("The Show"));
        assert_eq!(season.series_artwork.len(), 1);

        let episode = svc.episode(&member(), &EpisodeId("e9".into())).await.unwrap();
        assert_eq!(episode.series, Some(SeriesId("s9".into())));
        assert_eq!(episode.series_title.as_deref(), Some("The Show"));
        assert_eq!(episode.season_number, Some(0));
        assert_eq!(episode.season_title.as_deref(), Some("Specials"));
    }

    #[tokio::test]
    async fn an_orphaned_season_or_episode_is_not_found() {
        let catalog = MockCatalogRepo::new();
        catalog.add_season(Season {
            id: SeasonId("se9".into()),
            series: SeriesId("gone".into()),
            number: 1,
            title: None,
            overview: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog.add_episode(episode_numbered("e8", "se9", 1));
        catalog.add_version(episode_file("ev8", "e8", "lib1"));
        catalog.add_episode(Episode {
            id: EpisodeId("e9".into()),
            season: SeasonId("missing".into()),
            number: 1,
            title: "Orphan".into(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog.add_episode(episode_numbered("e7", "missing", 2));
        catalog.add_version(episode_file("ev7", "e7", "lib1"));
        let users = MockUserRepo::new();
        users.grant(&UserId("u1".into()), &[LibraryId("lib1".into())]);
        let svc = CatalogServiceImpl::new(catalog, users);

        let refused = [
            svc.season(&member(), &SeasonId("se9".into())).await.err(),
            svc.episodes(&member(), &SeasonId("se9".into())).await.err(),
            svc.episode(&member(), &EpisodeId("e8".into())).await.err(),
            svc.episode(&member(), &EpisodeId("e7".into())).await.err(),
        ];
        assert!(refused.iter().all(|error| matches!(error, Some(CatalogError::NotFound))));

        for unreachable in ["e7", "e8", "e9", "nobody"] {
            let title = TitleId::Episode(EpisodeId(unreachable.into()));
            assert!(matches!(
                svc.versions(&member(), &title, page()).await.unwrap_err(),
                CatalogError::NotFound
            ));
        }
    }

    fn titled(id: &str, title: TitleId, lib: &str) -> Version {
        Version { title, ..version(id, lib) }
    }

    fn granted_admin() -> MockUserRepo {
        let users = MockUserRepo::new();
        users.grant(&UserId("admin".into()), &[LibraryId("lib1".into())]);
        users
    }

    async fn collected() -> CatalogServiceImpl<MockCatalogRepo, MockUserRepo> {
        let svc = seeded().await;
        svc.catalog.add_movie(movie("m3", Some("R")));
        svc.catalog.add_version(titled("v3", TitleId::Movie(MovieId("m3".into())), "lib1"));
        svc.catalog.add_movie(movie("m4", Some("G")));
        svc.catalog.add_version(titled("v4", TitleId::Movie(MovieId("m4".into())), "lib2"));
        let sets: [(&str, &[&str]); 4] = [
            ("c-mixed", &["m1", "m3"]),
            ("c-rated", &["m3"]),
            ("c-elsewhere", &["m4"]),
            ("c-empty", &[]),
        ];
        for (id, members) in sets {
            svc.catalog.add_collection(Collection {
                id: CollectionId(id.into()),
                name: id.into(),
                overview: None,
                movies: members.iter().map(|movie| MovieId((*movie).into())).collect(),
                added_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
                artwork: Vec::new(),
            });
        }
        svc
    }

    fn collection_ids(page: &Page<Collection>) -> Vec<&str> {
        let mut ids: Vec<&str> = page.items.iter().map(|c| c.id.0.as_str()).collect();
        ids.sort();
        ids
    }

    fn member_ids(collection: &Collection) -> Vec<&str> {
        let mut ids: Vec<&str> = collection.movies.iter().map(|movie| movie.0.as_str()).collect();
        ids.sort();
        ids
    }

    fn limited(principal: Principal, age: u8) -> Principal {
        Principal { age_limit: Some(age), ..principal }
    }

    #[tokio::test]
    async fn collections_keep_to_the_cap_and_the_grants() {
        let svc = collected().await;

        let listed = svc.collections(&member(), page()).await.unwrap();
        assert_eq!(collection_ids(&listed), ["c-empty", "c-mixed"]);
        assert_eq!(listed.total, 2);
        let mixed = listed.items.iter().find(|c| c.id.0 == "c-mixed").unwrap();
        assert_eq!(member_ids(mixed), ["m1"]);

        assert_eq!(svc.collections(&admin(), page()).await.unwrap().total, 4);
        let child = svc.collections(&limited(admin(), 12), page()).await.unwrap();
        assert_eq!(collection_ids(&child), ["c-elsewhere", "c-empty"]);

        let first = svc.collections(&member(), PageRequest { offset: 0, limit: 1 }).await.unwrap();
        assert_eq!((first.items.len(), first.total), (1, 2));
    }

    #[tokio::test]
    async fn a_collection_and_a_movies_collections_show_only_what_the_viewer_may_see() {
        let svc = collected().await;

        let detail = svc.collection(&member(), &CollectionId("c-mixed".into())).await.unwrap();
        assert_eq!(member_ids(&detail.collection), ["m1"]);
        assert_eq!(detail.movies.iter().map(|m| m.id.0.as_str()).collect::<Vec<_>>(), ["m1"]);
        for hidden in ["c-rated", "c-elsewhere"] {
            assert!(matches!(
                svc.collection(&member(), &CollectionId(hidden.into())).await.unwrap_err(),
                CatalogError::NotFound
            ));
        }
        assert!(svc.collection(&member(), &CollectionId("c-empty".into())).await.is_ok());
        let full = svc.collection(&admin(), &CollectionId("c-mixed".into())).await.unwrap();
        assert_eq!(member_ids(&full.collection), ["m1", "m3"]);

        let of_m1 = svc.movie_collections(&member(), &MovieId("m1".into())).await.unwrap();
        assert_eq!(of_m1.len(), 1);
        assert_eq!(member_ids(&of_m1[0].collection), ["m1"]);
        assert!(svc.movie_collections(&member(), &MovieId("m3".into())).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn saving_a_collection_keeps_the_members_the_editor_cannot_see() {
        let svc = collected().await;
        let id = CollectionId("c-mixed".into());
        let update = |movies: Vec<MovieId>| CollectionUpdate {
            name: "Mixed".into(),
            overview: None,
            movies,
        };

        let saved = svc
            .update_collection(&limited(admin(), 16), &id, update(vec![MovieId("m1".into())]))
            .await
            .unwrap();
        assert_eq!(member_ids(&saved), ["m1", "m3"]);

        let cleared = svc.update_collection(&admin(), &id, update(Vec::new())).await.unwrap();
        assert!(cleared.movies.is_empty());
    }

    #[tokio::test]
    async fn versions_of_a_title_above_the_cap_are_not_found() {
        let svc = seeded().await;
        let m1 = TitleId::Movie(MovieId("m1".into()));
        let m3 = TitleId::Movie(MovieId("m3".into()));
        let e1 = TitleId::Episode(EpisodeId("e1".into()));
        svc.catalog.add_movie(movie("m3", Some("R")));
        svc.catalog.add_version(titled("v3", m3.clone(), "lib1"));
        svc.catalog.add_version(titled("ev1", e1.clone(), "lib1"));
        let child = limited(admin(), 12);

        for title in [&m3, &e1] {
            for refused in [member(), child.clone()] {
                assert!(matches!(
                    svc.versions(&refused, title, page()).await.unwrap_err(),
                    CatalogError::NotFound
                ));
            }
            assert_eq!(svc.versions(&admin(), title, page()).await.unwrap().total, 1);
        }
        assert!(svc.versions(&child, &m1, page()).await.is_err());
        assert_eq!(svc.versions(&member(), &m1, page()).await.unwrap().total, 1);
    }

    #[tokio::test]
    async fn versions_of_a_title_outside_the_grants_are_not_found_like_an_unknown_id() {
        let svc = seeded().await;
        let m4 = TitleId::Movie(MovieId("m4".into()));
        svc.catalog.add_movie(movie("m4", Some("G")));
        svc.catalog.add_version(titled("v4", m4.clone(), "lib2"));
        let e3 = TitleId::Episode(EpisodeId("e3".into()));
        svc.catalog.add_episode(episode_numbered("e3", "se2", 2));
        svc.catalog.add_version(episode_file("ev3", "e3", "lib2"));
        let unknown = TitleId::Movie(MovieId("x".into()));
        let versionless = TitleId::Movie(MovieId("m2".into()));

        for (caller, title) in [
            (member(), &m4),
            (member(), &e3),
            (member(), &unknown),
            (admin(), &unknown),
            (admin(), &versionless),
        ] {
            assert!(matches!(
                svc.versions(&caller, title, page()).await.unwrap_err(),
                CatalogError::NotFound
            ));
        }
        for granted in [&m4, &e3] {
            assert_eq!(svc.versions(&admin(), granted, page()).await.unwrap().total, 1);
        }
    }

    #[tokio::test]
    async fn a_watchable_version_keeps_to_the_grants_the_cap_and_the_age() {
        let svc = seeded().await;
        svc.catalog.add_movie(movie("m3", Some("R")));
        svc.catalog.add_version(titled("v3", TitleId::Movie(MovieId("m3".into())), "lib1"));
        svc.catalog.add_version(titled("ev1", TitleId::Episode(EpisodeId("e1".into())), "lib1"));

        for (caller, id) in [(member(), "v1"), (admin(), "v3"), (limited(admin(), 13), "v1")] {
            let detail = svc.watchable_version(&caller, &VersionId(id.into())).await.unwrap();
            assert_eq!(detail.version.id.0, id);
        }
        for (caller, id) in [
            (member(), "v2"),
            (member(), "v3"),
            (member(), "ev1"),
            (limited(admin(), 12), "v1"),
            (limited(admin(), 16), "v3"),
            (admin(), "x"),
        ] {
            assert!(
                matches!(
                    svc.watchable_version(&caller, &VersionId(id.into())).await.unwrap_err(),
                    CatalogError::NotFound
                ),
                "{id}"
            );
        }
        assert!(svc.version(&member(), &VersionId("v3".into())).await.is_ok());
    }

    #[tokio::test]
    async fn a_device_limit_narrows_every_account_and_never_lifts_a_cap() {
        let svc = seeded().await;
        svc.catalog.add_version(titled("vm2", TitleId::Movie(MovieId("m2".into())), "lib1"));
        let child = limited(admin(), 12);

        assert_eq!(svc.movies(&admin(), &query(), page()).await.unwrap().total, 2);
        assert_eq!(svc.movies(&child, &query(), page()).await.unwrap().total, 0);
        assert!(svc.movie(&child, &MovieId("m1".into())).await.is_err());
        assert!(svc.movie(&limited(member(), 18), &MovieId("m2".into())).await.is_err());
    }

    #[tokio::test]
    async fn collection_without_own_art_falls_back_to_member_posters() {
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(movie("m1", None));
        catalog.add_movie(movie("m2", None));
        catalog.add_version(version("v1", "lib1"));
        catalog.add_version(titled("v2", TitleId::Movie(MovieId("m2".into())), "lib1"));
        for id in ["m1", "m2"] {
            catalog
                .set_artwork(
                    &ArtworkOwner::Movie(MovieId(id.into())),
                    &[ArtworkRef {
                        id: ArtworkId(format!("{id}-poster")),
                        kind: ArtworkKind::Poster,
                        widths: vec![ArtworkWidth::new(180, format!("/art/{id}-poster/180.jpg"))],
                    }],
                )
                .await
                .unwrap();
        }
        catalog
            .upsert_collection(Collection {
                id: CollectionId("c1".into()),
                name: "Saga".into(),
                overview: None,
                movies: vec![MovieId("m1".into()), MovieId("m2".into())],
                added_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
                artwork: Vec::new(),
            })
            .await
            .unwrap();
        let svc = CatalogServiceImpl::new(catalog, granted_admin());

        let listed = svc.collections(&admin(), page()).await.unwrap();
        assert_eq!(listed.items[0].artwork.len(), 2);
        assert!(listed.items[0].artwork.iter().all(|art| art.kind == ArtworkKind::Poster));

        let detail = svc.collection(&admin(), &CollectionId("c1".into())).await.unwrap();
        assert_eq!(detail.collection.artwork.len(), 2);
    }

    #[tokio::test]
    async fn a_movie_reports_every_collection_it_belongs_to_by_name() {
        let catalog = MockCatalogRepo::new();
        for (id, name) in [("c-z", "Zulu Pack"), ("c-a", "alpha pack")] {
            catalog
                .upsert_collection(Collection {
                    id: CollectionId(id.into()),
                    name: name.into(),
                    overview: None,
                    movies: vec![MovieId("m1".into())],
                    added_at: Timestamp::UNIX_EPOCH,
                    updated_at: Timestamp::UNIX_EPOCH,
                    artwork: Vec::new(),
                })
                .await
                .unwrap();
        }
        catalog.add_movie(movie("m1", None));
        catalog.add_version(version("v1", "lib1"));
        let svc = CatalogServiceImpl::new(catalog, granted_admin());

        let found = svc.movie_collections(&admin(), &MovieId("m1".into())).await.unwrap();
        assert_eq!(
            found.iter().map(|d| d.collection.id.0.as_str()).collect::<Vec<_>>(),
            ["c-a", "c-z"],
            "collections come back ordered by name, case-insensitively, not by id"
        );

        assert!(
            svc.movie_collections(&admin(), &MovieId("nobody".into())).await.unwrap().is_empty()
        );
    }

    #[tokio::test]
    async fn title_cards_resolves_skips_missing_and_rating_gates() {
        let svc = seeded().await;
        svc.catalog.add_version(titled("vm2", TitleId::Movie(MovieId("m2".into())), "lib1"));
        svc.catalog.add_version(episode_file("ev1", "e1", "lib1"));
        let ids = vec![
            TitleId::Movie(MovieId("m1".into())),
            TitleId::Movie(MovieId("ghost".into())),
            TitleId::Episode(EpisodeId("e1".into())),
            TitleId::Movie(MovieId("m2".into())),
        ];

        let admin_cards = svc.title_cards(&admin(), &ids).await.unwrap();
        assert_eq!(card_ids(&admin_cards), ["m1", "e1", "m2"]);
        let TitleCard::Episode(e1) = &admin_cards[1] else { panic!("not an episode card") };
        assert_eq!(e1.series, Some(SeriesId("s1".into())));
        assert_eq!(e1.series_title.as_deref(), Some("s1"));
        assert_eq!(e1.season_number, Some(1));

        let member_cards = svc.title_cards(&member(), &ids).await.unwrap();
        assert_eq!(
            card_ids(&member_cards),
            ["m1"],
            "e1 sits under a TV-MA series, and the series rating is the only one an episode has"
        );

        assert!(svc.title_cards(&member(), &[]).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn every_episode_route_hides_a_blocked_series_from_a_capped_account() {
        let svc = seeded().await;
        svc.catalog.add_version(episode_file("ev1", "e1", "lib1"));

        let seasons = svc.seasons(&member(), &SeriesId("s1".into())).await.unwrap_err();
        assert!(
            matches!(seasons, CatalogError::NotFound),
            "listing the seasons of a blocked show enumerates it just as well"
        );
        let season = svc.season(&member(), &SeasonId("se1".into())).await.unwrap_err();
        assert!(
            matches!(season, CatalogError::NotFound),
            "the season card names the show and carries its artwork"
        );
        let episodes = svc.episodes(&member(), &SeasonId("se1".into())).await.unwrap_err();
        assert!(
            matches!(episodes, CatalogError::NotFound),
            "the episode list is the cheapest way to enumerate a blocked show"
        );
        let episode = svc.episode(&member(), &EpisodeId("e1".into())).await.unwrap_err();
        assert!(
            matches!(episode, CatalogError::NotFound),
            "gating the batch route while this one answers would be no gate at all"
        );
    }

    #[tokio::test]
    async fn an_unrated_series_is_hidden_under_a_cap() {
        let svc = serving(seeded_catalog_with(None).await).await;

        assert!(matches!(
            svc.series_detail(&member(), &SeriesId("s2".into())).await.unwrap_err(),
            CatalogError::NotFound
        ));
        assert!(matches!(
            svc.seasons(&member(), &SeriesId("s2".into())).await.unwrap_err(),
            CatalogError::NotFound
        ));
        assert!(matches!(
            svc.season(&member(), &SeasonId("se2".into())).await.unwrap_err(),
            CatalogError::NotFound
        ));
        assert!(matches!(
            svc.episodes(&member(), &SeasonId("se2".into())).await.unwrap_err(),
            CatalogError::NotFound
        ));
        assert!(
            matches!(
                svc.episode(&member(), &EpisodeId("e2".into())).await.unwrap_err(),
                CatalogError::NotFound
            ),
            "a show with no rating is hidden under a cap"
        );
        assert!(svc.episode(&admin(), &EpisodeId("e2".into())).await.is_ok());
    }

    #[tokio::test]
    async fn an_admin_reaches_a_blocked_series_through_every_episode_route() {
        let svc = seeded().await;
        svc.catalog.add_version(episode_file("ev1", "e1", "lib1"));

        assert_eq!(svc.seasons(&admin(), &SeriesId("s1".into())).await.unwrap().len(), 1);
        assert!(svc.season(&admin(), &SeasonId("se1".into())).await.is_ok());
        assert_eq!(svc.episodes(&admin(), &SeasonId("se1".into())).await.unwrap().len(), 1);
        assert!(svc.episode(&admin(), &EpisodeId("e1".into())).await.is_ok());
    }

    #[tokio::test]
    async fn title_cards_leave_out_orphaned_episodes() {
        let catalog = MockCatalogRepo::new();
        catalog.add_season(Season {
            id: SeasonId("se-orphan".into()),
            series: SeriesId("s-missing".into()),
            number: 4,
            title: None,
            overview: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog.add_episode(Episode {
            id: EpisodeId("no-series".into()),
            season: SeasonId("se-orphan".into()),
            number: 4,
            title: "no-series".into(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog.add_episode(Episode {
            id: EpisodeId("no-season".into()),
            season: SeasonId("gone".into()),
            number: 7,
            title: "no-season".into(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog.add_version(episode_file("v-no-series", "no-series", "lib1"));
        catalog.add_version(episode_file("v-no-season", "no-season", "lib1"));
        let svc = CatalogServiceImpl::new(catalog, granted_admin());

        let cards = svc
            .title_cards(
                &admin(),
                &[
                    TitleId::Episode(EpisodeId("no-series".into())),
                    TitleId::Episode(EpisodeId("no-season".into())),
                ],
            )
            .await
            .unwrap();

        assert!(cards.is_empty());
    }

    #[tokio::test]
    async fn title_cards_name_the_season_and_series_of_every_episode() {
        let catalog = MockCatalogRepo::new();
        catalog.add_series(Series {
            id: SeriesId("s1".into()),
            title: "Gamma".into(),
            sort_title: "gamma".into(),
            year: None,
            overview: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog.add_season(Season {
            id: SeasonId("se1".into()),
            series: SeriesId("s1".into()),
            number: 2,
            title: None,
            overview: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        for number in [1, 2] {
            catalog.add_episode(Episode {
                id: EpisodeId(format!("e{number}")),
                season: SeasonId("se1".into()),
                number,
                title: format!("e{number}"),
                overview: None,
                runtime_minutes: None,
                air_date: None,
                manually_edited: false,
                added_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
                artwork: Vec::new(),
            });
            catalog.add_version(episode_file(
                &format!("ev{number}"),
                &format!("e{number}"),
                "lib1",
            ));
        }
        let svc = CatalogServiceImpl::new(catalog, granted_admin());

        let cards = svc
            .title_cards(
                &admin(),
                &[
                    TitleId::Episode(EpisodeId("e1".into())),
                    TitleId::Episode(EpisodeId("ghost".into())),
                    TitleId::Episode(EpisodeId("e2".into())),
                ],
            )
            .await
            .unwrap();

        assert_eq!(
            card_ids(&cards),
            ["e1", "e2"],
            "an episode that no longer exists drops out of the batch"
        );
        for card in &cards {
            let TitleCard::Episode(episode) = card else {
                panic!("expected an episode card");
            };
            assert_eq!(episode.series_title.as_deref(), Some("Gamma"));
            assert_eq!(episode.season_number, Some(2));
        }
    }

    #[tokio::test]
    async fn person_filmography_genres_and_genre_filter() {
        let svc = seeded().await;
        let filed = seeded().await;
        filed.catalog.add_version(titled("vm2", TitleId::Movie(MovieId("m2".into())), "lib1"));
        filed.catalog.add_version(episode_file("ev1", "e1", "lib1"));

        // Admin sees the full filmography; the ghost-title credit is dropped.
        let admin_profile = filed.person(&admin(), &PersonId("p1".into())).await.unwrap();
        assert_eq!(admin_profile.person.name, "Ada");
        assert_eq!(
            admin_profile.filmography.iter().map(|e| e.title.id()).collect::<Vec<_>>(),
            ["m1", "m2", "s1"]
        );
        assert_eq!(admin_profile.filmography[0].display_title, "m1");
        assert_eq!(admin_profile.filmography[0].character, Some("Hero".to_owned()));

        // Member (PG-13 cap) keeps only the PG-13 movie; R and TV-MA drop out.
        let member_profile = filed.person(&member(), &PersonId("p1".into())).await.unwrap();
        assert_eq!(
            member_profile.filmography.iter().map(|e| e.title.id()).collect::<Vec<_>>(),
            ["m1"]
        );

        let p2 = svc.person(&admin(), &PersonId("p2".into())).await.unwrap();
        assert_eq!(p2.filmography.len(), 1);
        assert_eq!(p2.filmography[0].role, CreditRole::Director);

        assert!(matches!(
            svc.person(&admin(), &PersonId("nope".into())).await.unwrap_err(),
            CatalogError::NotFound
        ));

        // Genres (deduped, name-sorted by the mock), optionally scoped to one kind.
        let names =
            |genres: Vec<Genre>| genres.iter().map(|g| g.name.clone()).collect::<Vec<String>>();
        assert_eq!(names(svc.genres(&admin(), None).await.unwrap()), ["Action", "Drama"]);
        assert_eq!(names(svc.genres(&admin(), Some(TitleKind::Series)).await.unwrap()), ["Action"]);

        // Genre filter: m2 has no version, so no list reaches it whoever asks.
        let action = "Action";
        let admin_action = svc.movies(&admin(), &genre_query(action), page()).await.unwrap();
        assert_eq!(admin_action.items.iter().map(|m| m.id.0.as_str()).collect::<Vec<_>>(), ["m1"]);
        let member_action = svc.movies(&member(), &genre_query(action), page()).await.unwrap();
        assert_eq!(member_action.items.iter().map(|m| m.id.0.as_str()).collect::<Vec<_>>(), ["m1"]);

        let series_action = svc.series(&admin(), &genre_query(action), page()).await.unwrap();
        assert_eq!(
            series_action.items.iter().map(|s| s.series.id.0.as_str()).collect::<Vec<_>>(),
            ["s2"],
            "s1 carries the genre too, but has no file and so sits in no library"
        );

        let detail = svc.movie(&admin(), &MovieId("m1".into())).await.unwrap();
        assert_eq!(detail.genres.len(), 1);
        assert_eq!(detail.credits.len(), 2);
        let series_detail = filed.series_detail(&admin(), &SeriesId("s1".into())).await.unwrap();
        assert_eq!(series_detail.credits.len(), 1);
    }

    fn scoped_version(id: &str, title: TitleId, lib: &str, quality: Quality) -> Version {
        Version {
            id: VersionId(id.into()),
            title,
            library: LibraryId(lib.into()),
            quality,
            container: "mkv".into(),
            path: format!("/media/{id}.mkv"),
            size_bytes: 1,
            duration_ms: 1000,
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    async fn random_fixture() -> CatalogServiceImpl<MockCatalogRepo, MockUserRepo> {
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(movie("m1", Some("PG-13")));
        catalog.add_movie(movie("adult", Some("R")));
        catalog.add_version(scoped_version(
            "hd-lib1",
            TitleId::Movie(MovieId("m1".into())),
            "lib1",
            Quality::Hd,
        ));
        catalog.add_version(scoped_version(
            "uhd-lib2",
            TitleId::Movie(MovieId("m1".into())),
            "lib2",
            Quality::Uhd,
        ));
        catalog.add_version(scoped_version(
            "adult-v",
            TitleId::Movie(MovieId("adult".into())),
            "lib1",
            Quality::Hd,
        ));

        let users = MockUserRepo::new();
        users.insert(capped_user());
        users.set_library_access(&UserId("u1".into()), &[LibraryId("lib1".into())]).await.unwrap();
        users.grant(&UserId("admin".into()), &[LibraryId("lib1".into()), LibraryId("lib2".into())]);
        CatalogServiceImpl::new(catalog, users)
    }

    #[tokio::test]
    async fn a_random_pick_never_reaches_past_the_reader_library_access() {
        let svc = random_fixture().await;

        assert_eq!(
            svc.random(&admin(), &RandomScope::Movies, &query()).await.unwrap(),
            VersionId("uhd-lib2".into()),
            "this account holds both libraries, so the best copy wins outright"
        );
        assert_eq!(
            svc.random(&member(), &RandomScope::Movies, &query()).await.unwrap(),
            VersionId("hd-lib1".into()),
            "the 4K copy is in a library this reader cannot open, so the HD copy is its best"
        );
    }

    #[tokio::test]
    async fn a_random_pick_obeys_the_rating_cap() {
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(movie("adult", Some("R")));
        catalog.add_version(scoped_version(
            "adult-v",
            TitleId::Movie(MovieId("adult".into())),
            "lib1",
            Quality::Hd,
        ));
        let users = MockUserRepo::new();
        users.insert(capped_user());
        users.set_library_access(&UserId("u1".into()), &[LibraryId("lib1".into())]).await.unwrap();
        users.grant(&UserId("admin".into()), &[LibraryId("lib1".into())]);
        let svc = CatalogServiceImpl::new(catalog, users);

        assert!(matches!(
            svc.random(&member(), &RandomScope::Movies, &query()).await,
            Err(CatalogError::NotFound)
        ));
        assert!(svc.random(&admin(), &RandomScope::Movies, &query()).await.is_ok());
    }

    #[tokio::test]
    async fn a_random_pick_with_nothing_playable_is_not_found() {
        let svc = seeded().await;

        assert!(matches!(
            svc.random(&member(), &RandomScope::Season(SeasonId("se1".into())), &query()).await,
            Err(CatalogError::NotFound)
        ));
        assert!(matches!(
            svc.random(&member(), &RandomScope::Collection(CollectionId("nope".into())), &query())
                .await,
            Err(CatalogError::NotFound)
        ));
    }

    #[tokio::test]
    async fn a_random_episode_stays_inside_the_scope_it_was_asked_for() {
        let svc = seeded().await;

        assert_eq!(
            svc.random(&admin(), &RandomScope::Season(SeasonId("se2".into())), &query())
                .await
                .unwrap(),
            VersionId("ev2".into())
        );
        assert_eq!(
            svc.random(&admin(), &RandomScope::Series(SeriesId("s2".into())), &query())
                .await
                .unwrap(),
            VersionId("ev2".into())
        );
        assert!(
            matches!(
                svc.random(&admin(), &RandomScope::Series(SeriesId("s1".into())), &query()).await,
                Err(CatalogError::NotFound)
            ),
            "s1 has an episode but no file behind it"
        );
    }
}

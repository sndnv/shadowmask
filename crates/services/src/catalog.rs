use std::collections::HashMap;

use domain::catalog::{
    ArtworkRef, Collection, CollectionDetail, CollectionId, CollectionUpdate, Episode, EpisodeCard,
    EpisodeId, FilmographyEntry, Movie, MovieDetail, MovieId, NewCollection, PersonProfile,
    RandomScope, Season, SeasonCard, SeasonId, Series, SeriesDetail, SeriesId, TitleCard, TitleId,
    TitleKind, TitleListFilter, TitleListQuery, TitleRef, Version, VersionDetail, VersionId,
    best_available, in_year_order,
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
use crate::page::paginate;

const MOSAIC_TILES: usize = 4;

fn collection_poster(movie: &Movie) -> Option<ArtworkRef> {
    movie
        .artwork
        .iter()
        .find(|art| art.kind == ArtworkKind::Poster)
        .cloned()
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
    C: CatalogRepository + Sync,
    U: UserRepository + Sync,
{
    async fn sorted_members(&self, members: &[MovieId]) -> Result<Vec<MovieId>, CatalogError> {
        let known = self.catalog.movies_by_ids(members).await?;
        Ok(in_year_order(members, &known))
    }

    async fn viewer(&self, caller: &Principal) -> Result<acl::Viewer, CatalogError> {
        Ok(acl::viewer(&self.users, &caller.user).await?)
    }

    async fn collection_mosaic_art(
        &self,
        movies: &[MovieId],
    ) -> Result<Vec<ArtworkRef>, CatalogError> {
        let tiles = &movies[..movies.len().min(MOSAIC_TILES)];
        Ok(self
            .catalog
            .movies_by_ids(tiles)
            .await?
            .iter()
            .filter_map(collection_poster)
            .collect())
    }

    async fn permits_series(
        &self,
        caller: &Principal,
        series: Option<&Series>,
    ) -> Result<bool, CatalogError> {
        Ok(self
            .viewer(caller)
            .await?
            .permits(series.and_then(|show| show.content_rating.as_ref())))
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
    C: CatalogRepository + Sync,
    U: UserRepository + Sync,
{
    async fn collections(
        &self,
        _caller: &Principal,
        page: PageRequest,
    ) -> Result<Page<Collection>, CatalogError> {
        let mut page = self.catalog.list_collections(page).await?;
        for collection in page.items.iter_mut() {
            if collection.artwork.is_empty() {
                collection.artwork = self.collection_mosaic_art(&collection.movies).await?;
            }
        }
        Ok(page)
    }

    async fn collection(
        &self,
        _caller: &Principal,
        id: &CollectionId,
    ) -> Result<CollectionDetail, CatalogError> {
        let mut collection = self
            .catalog
            .get_collection(id)
            .await?
            .ok_or(CatalogError::NotFound)?;
        let movies = self.catalog.movies_by_ids(&collection.movies).await?;
        if collection.artwork.is_empty() {
            collection.artwork = movies
                .iter()
                .take(MOSAIC_TILES)
                .filter_map(collection_poster)
                .collect();
        }
        Ok(CollectionDetail { collection, movies })
    }

    async fn movie_collections(
        &self,
        caller: &Principal,
        movie: &MovieId,
    ) -> Result<Vec<CollectionDetail>, CatalogError> {
        let ids = self.catalog.collections_of_movie(movie).await?;
        let mut found = Vec::with_capacity(ids.len());
        for id in &ids {
            match self.collection(caller, id).await {
                Ok(detail) => found.push(detail),
                Err(CatalogError::NotFound) => continue,
                Err(err) => return Err(err),
            }
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
        _caller: &Principal,
        id: &CollectionId,
        update: CollectionUpdate,
    ) -> Result<Collection, CatalogError> {
        self.catalog
            .get_collection(id)
            .await?
            .ok_or(CatalogError::NotFound)?;
        let now = Timestamp::now();
        let collection = Collection {
            id: id.clone(),
            name: update.name,
            overview: update.overview,
            movies: self.sorted_members(&update.movies).await?,
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
        self.catalog
            .get_collection(id)
            .await?
            .ok_or(CatalogError::NotFound)?;
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
        let detail = self
            .catalog
            .movie_detail(id)
            .await?
            .ok_or(CatalogError::NotFound)?;
        if !self
            .viewer(caller)
            .await?
            .permits(detail.movie.content_rating.as_ref())
        {
            return Err(CatalogError::NotFound);
        }
        Ok(detail)
    }

    async fn series(
        &self,
        caller: &Principal,
        query: &TitleListQuery,
        page: PageRequest,
    ) -> Result<Page<Series>, CatalogError> {
        let filter = self.build_filter(caller, query).await?;
        Ok(self.catalog.list_series_filtered(&filter, page).await?)
    }

    async fn series_detail(
        &self,
        caller: &Principal,
        id: &SeriesId,
    ) -> Result<SeriesDetail, CatalogError> {
        let detail = self
            .catalog
            .series_detail(id)
            .await?
            .ok_or(CatalogError::NotFound)?;
        if !self
            .viewer(caller)
            .await?
            .permits(detail.series.content_rating.as_ref())
        {
            return Err(CatalogError::NotFound);
        }
        Ok(detail)
    }

    async fn seasons(
        &self,
        caller: &Principal,
        series: &SeriesId,
    ) -> Result<Vec<Season>, CatalogError> {
        let viewer = self.viewer(caller).await?;
        let show = self.catalog.get_series(series).await?;
        if !viewer.permits(show.as_ref().and_then(|show| show.content_rating.as_ref())) {
            return Err(CatalogError::NotFound);
        }
        Ok(self.catalog.list_seasons(series).await?)
    }

    async fn season(&self, caller: &Principal, id: &SeasonId) -> Result<SeasonCard, CatalogError> {
        let season = self
            .catalog
            .get_season(id)
            .await?
            .ok_or(CatalogError::NotFound)?;
        let series = self.catalog.get_series(&season.series).await?;
        if !self.permits_series(caller, series.as_ref()).await? {
            return Err(CatalogError::NotFound);
        }
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
        let series = match self.catalog.get_season(season).await? {
            Some(season) => self.catalog.get_series(&season.series).await?,
            None => None,
        };
        if !viewer.permits(
            series
                .as_ref()
                .and_then(|show| show.content_rating.as_ref()),
        ) {
            return Err(CatalogError::NotFound);
        }
        Ok(self.catalog.list_episodes(season).await?)
    }

    async fn episode(
        &self,
        caller: &Principal,
        id: &EpisodeId,
    ) -> Result<EpisodeCard, CatalogError> {
        let episode = self
            .catalog
            .get_episode(id)
            .await?
            .ok_or(CatalogError::NotFound)?;
        let season = self.catalog.get_season(&episode.season).await?;
        let series = match &season {
            Some(season) => self.catalog.get_series(&season.series).await?,
            None => None,
        };
        if !self.permits_series(caller, series.as_ref()).await? {
            return Err(CatalogError::NotFound);
        }
        Ok(EpisodeCard {
            episode,
            series: season.as_ref().map(|s| s.series.clone()),
            series_title: series.as_ref().map(|s| s.title.clone()),
            series_artwork: series.map(|s| s.artwork).unwrap_or_default(),
            season_number: season.as_ref().map(|s| s.number),
            season_title: season.and_then(|s| s.title),
        })
    }

    async fn versions(
        &self,
        caller: &Principal,
        title: &TitleId,
        page: PageRequest,
    ) -> Result<Page<Version>, CatalogError> {
        let viewer = self.viewer(caller).await?;
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
        page: PageRequest,
    ) -> Result<Page<Version>, CatalogError> {
        if !acl::is_admin(caller) {
            return Err(CatalogError::Forbidden);
        }
        let viewer = self.viewer(caller).await?;
        let filtered: Vec<Version> = self
            .catalog
            .list_all_versions(PageRequest::ALL)
            .await?
            .items
            .into_iter()
            .filter(|version| viewer.sees_library(&version.library))
            .collect();
        Ok(paginate(&filtered, page))
    }

    async fn version(
        &self,
        caller: &Principal,
        id: &VersionId,
    ) -> Result<VersionDetail, CatalogError> {
        let detail = self
            .catalog
            .version_detail(id)
            .await?
            .ok_or(CatalogError::NotFound)?;
        if !self
            .viewer(caller)
            .await?
            .sees_library(&detail.version.library)
        {
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
        let person = self
            .catalog
            .get_person(id)
            .await?
            .ok_or(CatalogError::NotFound)?;
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
        let movies: HashMap<MovieId, Movie> = self
            .catalog
            .movies_by_ids(&movie_ids)
            .await?
            .into_iter()
            .map(|movie| (movie.id.clone(), movie))
            .collect();
        let series: HashMap<SeriesId, Series> = self
            .catalog
            .series_by_ids(&series_ids)
            .await?
            .into_iter()
            .map(|show| (show.id.clone(), show))
            .collect();

        let mut filmography = Vec::new();
        for credit in credits {
            let resolved = match &credit.title {
                TitleRef::Movie(movie) => movies
                    .get(movie)
                    .map(|m| (&m.title, m.year, &m.artwork, &m.content_rating)),
                TitleRef::Series(show) => series
                    .get(show)
                    .map(|s| (&s.title, s.year, &s.artwork, &s.content_rating)),
            };
            let Some((display_title, year, artwork, content_rating)) = resolved else {
                continue;
            };
            let (display_title, artwork) = (display_title.clone(), artwork.clone());
            if !viewer.permits(content_rating.as_ref()) {
                continue;
            }
            filmography.push(FilmographyEntry {
                title: credit.title,
                display_title,
                year,
                artwork,
                role: credit.role,
                character: credit.character,
            });
        }
        Ok(PersonProfile {
            person,
            filmography,
        })
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
        let viewer = self.viewer(caller).await?;
        let mut seasons: HashMap<SeasonId, Option<Season>> = HashMap::new();
        let mut series: HashMap<SeriesId, Option<Series>> = HashMap::new();
        let mut cards = Vec::new();
        for id in ids {
            match id {
                TitleId::Movie(movie_id) => {
                    let Some(movie) = self.catalog.get_movie(movie_id).await? else {
                        continue;
                    };
                    if !viewer.permits(movie.content_rating.as_ref()) {
                        continue;
                    }
                    cards.push(TitleCard::Movie(movie));
                }
                TitleId::Episode(episode_id) => {
                    let Some(episode) = self.catalog.get_episode(episode_id).await? else {
                        continue;
                    };
                    let season = match seasons.get(&episode.season).cloned() {
                        Some(cached) => cached,
                        None => {
                            let loaded = self.catalog.get_season(&episode.season).await?;
                            seasons.insert(episode.season.clone(), loaded.clone());
                            loaded
                        }
                    };
                    let series_id = season.as_ref().map(|s| s.series.clone());
                    let resolved = match &series_id {
                        Some(sid) => match series.get(sid).cloned() {
                            Some(cached) => cached,
                            None => {
                                let loaded = self.catalog.get_series(sid).await?;
                                series.insert(sid.clone(), loaded.clone());
                                loaded
                            }
                        },
                        None => None,
                    };
                    if !viewer.permits(resolved.as_ref().and_then(|s| s.content_rating.as_ref())) {
                        continue;
                    }
                    cards.push(TitleCard::Episode(EpisodeCard {
                        season_number: season.as_ref().map(|s| s.number),
                        season_title: season.and_then(|s| s.title),
                        series: series_id,
                        series_title: resolved.as_ref().map(|s| s.title.clone()),
                        series_artwork: resolved.map(|s| s.artwork).unwrap_or_default(),
                        episode,
                    }));
                }
            }
        }
        Ok(cards)
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
        Genre {
            id: GenreId(id.into()),
            name: name.into(),
        }
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
        }
    }

    fn member() -> Principal {
        Principal {
            user: UserId("u1".into()),
            role: Role::User,
        }
    }

    fn page() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 10,
        }
    }

    fn query() -> TitleListQuery {
        TitleListQuery::default()
    }

    fn genre_query(genre: &str) -> TitleListQuery {
        TitleListQuery {
            genres: vec![genre.to_owned()],
            ..TitleListQuery::default()
        }
    }

    fn library_query(library: &str) -> TitleListQuery {
        TitleListQuery {
            library: Some(LibraryId(library.into())),
            ..TitleListQuery::default()
        }
    }

    fn rating(code: &str) -> ContentRating {
        ContentRating {
            system: "MPAA".into(),
            code: code.into(),
        }
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
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(movie("m1", Some("PG-13")));
        catalog.add_movie(movie("m2", Some("R")));
        catalog.add_series(Series {
            id: SeriesId("s1".into()),
            title: "s1".into(),
            sort_title: "s1".into(),
            year: None,
            overview: None,
            content_rating: Some(ContentRating {
                system: "US-TV".into(),
                code: "TV-MA".into(),
            }),
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
            content_rating: None,
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
        let catalog = seeded_catalog().await;
        let users = MockUserRepo::new();
        users.insert(capped_user());
        users
            .set_library_access(&UserId("u1".into()), &[LibraryId("lib1".into())])
            .await
            .unwrap();
        // Admin carries no implicit access, so it is granted both libraries here
        // the same way a real admin would grant itself.
        users.grant(
            &UserId("admin".into()),
            &[LibraryId("lib1".into()), LibraryId("lib2".into())],
        );
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
        assert!(svc.movie(&admin(), &MovieId("m2".into())).await.is_ok());
        assert_eq!(
            svc.series(&admin(), &query(), page()).await.unwrap().total,
            1,
            "s1 has no file, so like m2 it belongs to no library"
        );
        assert!(
            svc.series_detail(&admin(), &SeriesId("s1".into()))
                .await
                .is_ok()
        );
        assert_eq!(
            svc.versions(&admin(), &TitleId::Movie(MovieId("m1".into())), page())
                .await
                .unwrap()
                .total,
            2
        );
        assert_eq!(
            svc.library_versions(&admin(), &LibraryId("lib2".into()), page())
                .await
                .unwrap()
                .total,
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
        };

        assert_eq!(
            svc.movies(&stranger, &query(), page()).await.unwrap().total,
            0
        );
        assert_eq!(
            svc.versions(&stranger, &TitleId::Movie(MovieId("m1".into())), page())
                .await
                .unwrap()
                .total,
            0
        );
        assert!(matches!(
            svc.version(&stranger, &VersionId("v1".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));
    }

    #[tokio::test]
    async fn all_versions_is_admin_only_and_still_scoped_to_the_grants() {
        let svc = seeded().await;
        let all = svc.all_versions(&admin(), page()).await.unwrap();
        assert_eq!(all.total, 3);
        assert!(matches!(
            svc.all_versions(&member(), page()).await.unwrap_err(),
            CatalogError::Forbidden
        ));

        let stranger = Principal {
            user: UserId("nobody".into()),
            role: Role::Admin,
        };
        assert_eq!(
            svc.all_versions(&stranger, page()).await.unwrap().total,
            0,
            "an admin listing must not show a version whose detail page would 404"
        );
    }

    #[tokio::test]
    async fn member_rating_gate_filters_and_hides() {
        let svc = seeded().await;
        let movies = svc.movies(&member(), &query(), page()).await.unwrap();
        assert_eq!(movies.total, 1);
        assert_eq!(movies.items[0].id, MovieId("m1".into()));
        assert!(svc.movie(&member(), &MovieId("m1".into())).await.is_ok());
        assert!(matches!(
            svc.movie(&member(), &MovieId("m2".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));

        let series = svc.series(&member(), &query(), page()).await.unwrap();
        assert_eq!(series.total, 1);
        assert_eq!(series.items[0].id, SeriesId("s2".into()));
        assert!(
            svc.series_detail(&member(), &SeriesId("s2".into()))
                .await
                .is_ok()
        );
        assert!(matches!(
            svc.series_detail(&member(), &SeriesId("s1".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));
    }

    #[tokio::test]
    async fn list_scoping_and_sort() {
        let svc = seeded().await;

        let admin_lib1 = svc
            .movies(&admin(), &library_query("lib1"), page())
            .await
            .unwrap();
        assert_eq!(
            admin_lib1
                .items
                .iter()
                .map(|m| m.id.0.as_str())
                .collect::<Vec<_>>(),
            ["m1"]
        );
        let admin_series_lib1 = svc
            .series(&admin(), &library_query("lib1"), page())
            .await
            .unwrap();
        assert_eq!(
            admin_series_lib1
                .items
                .iter()
                .map(|s| s.id.0.as_str())
                .collect::<Vec<_>>(),
            ["s2"]
        );
        assert!(
            svc.movies(&admin(), &library_query("ghost"), page())
                .await
                .unwrap()
                .items
                .is_empty()
        );

        assert!(
            svc.movies(&member(), &library_query("lib2"), page())
                .await
                .unwrap()
                .items
                .is_empty()
        );
        let member_lib1 = svc
            .movies(&member(), &library_query("lib1"), page())
            .await
            .unwrap();
        assert_eq!(
            member_lib1
                .items
                .iter()
                .map(|m| m.id.0.as_str())
                .collect::<Vec<_>>(),
            ["m1"]
        );
        let member_series_lib1 = svc
            .series(&member(), &library_query("lib1"), page())
            .await
            .unwrap();
        assert_eq!(
            member_series_lib1
                .items
                .iter()
                .map(|s| s.id.0.as_str())
                .collect::<Vec<_>>(),
            ["s2"]
        );

        let by_title_desc = TitleListQuery {
            sort: TitleSort::Title,
            order: SortOrder::Desc,
            ..TitleListQuery::default()
        };
        let sorted = svc.movies(&admin(), &by_title_desc, page()).await.unwrap();
        assert_eq!(
            sorted
                .items
                .iter()
                .map(|m| m.id.0.as_str())
                .collect::<Vec<_>>(),
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
            svc.library_versions(&member(), &LibraryId("lib1".into()), page())
                .await
                .unwrap()
                .total,
            2
        );
        assert_eq!(
            svc.library_versions(&member(), &LibraryId("lib2".into()), page())
                .await
                .unwrap()
                .total,
            0
        );

        assert!(
            svc.version(&member(), &VersionId("v1".into()))
                .await
                .is_ok()
        );
        assert!(matches!(
            svc.version(&member(), &VersionId("v2".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));
    }

    #[tokio::test]
    async fn seasons_episodes_and_missing() {
        let svc = seeded().await;
        assert_eq!(
            svc.seasons(&member(), &SeriesId("s2".into()))
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(svc.season(&member(), &SeasonId("se2".into())).await.is_ok());
        assert!(matches!(
            svc.season(&member(), &SeasonId("x".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));
        assert_eq!(
            svc.episodes(&member(), &SeasonId("se2".into()))
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(
            svc.episode(&member(), &EpisodeId("e2".into()))
                .await
                .is_ok()
        );
        assert!(matches!(
            svc.episode(&member(), &EpisodeId("x".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));
        assert!(matches!(
            svc.movie(&member(), &MovieId("x".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));
        assert!(matches!(
            svc.version(&member(), &VersionId("x".into()))
                .await
                .unwrap_err(),
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
                NewCollection {
                    name: "Saga".into(),
                    overview: None,
                    movies: members.clone(),
                },
            )
            .await
            .unwrap();
        assert_eq!(
            created.movies,
            vec![MovieId("early".into()), MovieId("late".into())]
        );

        let updated = svc
            .update_collection(
                &admin(),
                &created.id,
                CollectionUpdate {
                    name: "Saga".into(),
                    overview: None,
                    movies: members,
                },
            )
            .await
            .unwrap();
        assert_eq!(
            updated.movies,
            vec![MovieId("early".into()), MovieId("late".into())]
        );
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
            svc.collection(&admin(), &CollectionId("x".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));

        let updated = svc
            .update_collection(
                &admin(),
                &created.id,
                CollectionUpdate {
                    name: "Saga II".into(),
                    overview: None,
                    movies: Vec::new(),
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.name, "Saga II");
        assert!(matches!(
            svc.update_collection(
                &admin(),
                &CollectionId("x".into()),
                CollectionUpdate {
                    name: "x".into(),
                    overview: None,
                    movies: Vec::new(),
                },
            )
            .await
            .unwrap_err(),
            CatalogError::NotFound
        ));

        svc.delete_collection(&admin(), &created.id).await.unwrap();
        assert!(matches!(
            svc.delete_collection(&admin(), &created.id)
                .await
                .unwrap_err(),
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
        users.grant(
            &UserId("admin".into()),
            &[LibraryId("lib1".into()), LibraryId("lib2".into())],
        );
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
                assert!(matches!(
                    $call.await.unwrap_err(),
                    CatalogError::Repository(_)
                ))
            };
        }

        is_repository_error!(svc.collections(&admin(), page()));
        is_repository_error!(svc.collection(&admin(), &collection));
        is_repository_error!(svc.movie_collections(&admin(), &movie));
        is_repository_error!(svc.create_collection(
            &admin(),
            NewCollection {
                name: "Saga".into(),
                overview: None,
                movies: vec![movie.clone()],
            }
        ));
        is_repository_error!(svc.update_collection(
            &admin(),
            &collection,
            CollectionUpdate {
                name: "Saga".into(),
                overview: None,
                movies: vec![movie.clone()],
            }
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
        is_repository_error!(svc.all_versions(&admin(), page()));
        is_repository_error!(svc.version(&admin(), &version));
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
                assert!(matches!(
                    $call.await.unwrap_err(),
                    CatalogError::Repository(_)
                ))
            };
        }

        is_repository_error!(svc.movies(&admin(), &query(), page()));
        is_repository_error!(svc.series(&admin(), &query(), page()));
        is_repository_error!(svc.seasons(&admin(), &series));
        is_repository_error!(svc.episodes(&admin(), &season));
        is_repository_error!(svc.versions(&admin(), &title, page()));
        is_repository_error!(svc.library_versions(&admin(), &library, page()));
        is_repository_error!(svc.all_versions(&admin(), page()));
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
        let svc = CatalogServiceImpl::new(catalog, MockUserRepo::new());

        let season = svc
            .season(&member(), &SeasonId("se9".into()))
            .await
            .unwrap();
        assert_eq!(season.series_title.as_deref(), Some("The Show"));
        assert_eq!(season.series_artwork.len(), 1);

        let episode = svc
            .episode(&member(), &EpisodeId("e9".into()))
            .await
            .unwrap();
        assert_eq!(episode.series, Some(SeriesId("s9".into())));
        assert_eq!(episode.series_title.as_deref(), Some("The Show"));
        assert_eq!(episode.season_number, Some(0));
        assert_eq!(episode.season_title.as_deref(), Some("Specials"));
    }

    #[tokio::test]
    async fn an_orphaned_season_or_episode_reports_no_context() {
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
        let svc = CatalogServiceImpl::new(catalog, MockUserRepo::new());

        let season = svc
            .season(&member(), &SeasonId("se9".into()))
            .await
            .unwrap();
        assert!(season.series_title.is_none());
        assert!(season.series_artwork.is_empty());

        let episode = svc
            .episode(&member(), &EpisodeId("e9".into()))
            .await
            .unwrap();
        assert!(episode.series.is_none());
        assert!(episode.series_title.is_none());
        assert!(episode.season_number.is_none());
        assert!(episode.season_title.is_none());
    }

    #[tokio::test]
    async fn collection_without_own_art_falls_back_to_member_posters() {
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(movie("m1", None));
        catalog.add_movie(movie("m2", None));
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
        let svc = CatalogServiceImpl::new(catalog, MockUserRepo::new());

        let listed = svc.collections(&admin(), page()).await.unwrap();
        assert_eq!(listed.items[0].artwork.len(), 2);
        assert!(
            listed.items[0]
                .artwork
                .iter()
                .all(|art| art.kind == ArtworkKind::Poster)
        );

        let detail = svc
            .collection(&admin(), &CollectionId("c1".into()))
            .await
            .unwrap();
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
        let svc = CatalogServiceImpl::new(catalog, MockUserRepo::new());

        let found = svc
            .movie_collections(&admin(), &MovieId("m1".into()))
            .await
            .unwrap();
        assert_eq!(
            found
                .iter()
                .map(|d| d.collection.id.0.as_str())
                .collect::<Vec<_>>(),
            ["c-a", "c-z"],
            "collections come back ordered by name, case-insensitively, not by id"
        );

        assert!(
            svc.movie_collections(&admin(), &MovieId("nobody".into()))
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn title_cards_resolves_skips_missing_and_rating_gates() {
        let svc = seeded().await;
        let ids = vec![
            TitleId::Movie(MovieId("m1".into())),
            TitleId::Movie(MovieId("ghost".into())),
            TitleId::Episode(EpisodeId("e1".into())),
            TitleId::Movie(MovieId("m2".into())),
        ];

        let admin_cards = svc.title_cards(&admin(), &ids).await.unwrap();
        assert_eq!(card_ids(&admin_cards), ["m1", "e1", "m2"]);
        let TitleCard::Episode(e1) = &admin_cards[1] else {
            panic!("expected an episode card");
        };
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

        assert!(
            matches!(
                svc.seasons(&member(), &SeriesId("s1".into()))
                    .await
                    .unwrap_err(),
                CatalogError::NotFound
            ),
            "listing the seasons of a blocked show enumerates it just as well"
        );
        assert!(
            matches!(
                svc.season(&member(), &SeasonId("se1".into()))
                    .await
                    .unwrap_err(),
                CatalogError::NotFound
            ),
            "the season card names the show and carries its artwork"
        );
        assert!(
            matches!(
                svc.episodes(&member(), &SeasonId("se1".into()))
                    .await
                    .unwrap_err(),
                CatalogError::NotFound
            ),
            "the episode list is the cheapest way to enumerate a blocked show"
        );
        assert!(
            matches!(
                svc.episode(&member(), &EpisodeId("e1".into()))
                    .await
                    .unwrap_err(),
                CatalogError::NotFound
            ),
            "gating the batch route while this one answers would be no gate at all"
        );
    }

    #[tokio::test]
    async fn an_unrated_series_stays_reachable_under_a_cap() {
        let svc = seeded().await;

        assert_eq!(
            svc.seasons(&member(), &SeriesId("s2".into()))
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            svc.season(&member(), &SeasonId("se2".into()))
                .await
                .unwrap()
                .season
                .number,
            1
        );
        assert_eq!(
            svc.episodes(&member(), &SeasonId("se2".into()))
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            svc.episode(&member(), &EpisodeId("e2".into()))
                .await
                .unwrap()
                .episode
                .id,
            EpisodeId("e2".into()),
            "a show with no rating is not a show above the cap"
        );
    }

    #[tokio::test]
    async fn an_admin_reaches_a_blocked_series_through_every_episode_route() {
        let svc = seeded().await;

        assert_eq!(
            svc.seasons(&admin(), &SeriesId("s1".into()))
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(svc.season(&admin(), &SeasonId("se1".into())).await.is_ok());
        assert_eq!(
            svc.episodes(&admin(), &SeasonId("se1".into()))
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(svc.episode(&admin(), &EpisodeId("e1".into())).await.is_ok());
    }

    #[tokio::test]
    async fn title_cards_episode_context_falls_back_when_rows_missing() {
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
        let svc = CatalogServiceImpl::new(catalog, MockUserRepo::new());

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

        let TitleCard::Episode(no_series) = &cards[0] else {
            panic!("expected an episode card");
        };
        assert_eq!(no_series.series, Some(SeriesId("s-missing".into())));
        assert_eq!(no_series.series_title, None);
        assert_eq!(no_series.season_number, Some(4));

        let TitleCard::Episode(no_season) = &cards[1] else {
            panic!("expected an episode card");
        };
        assert_eq!(no_season.series, None);
        assert_eq!(no_season.series_title, None);
        assert_eq!(no_season.season_number, None);
    }

    #[tokio::test]
    async fn title_cards_reuse_the_season_and_series_they_already_loaded() {
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
        }
        let svc = CatalogServiceImpl::new(catalog, MockUserRepo::new());

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

        // Admin sees the full filmography; the ghost-title credit is dropped.
        let admin_profile = svc.person(&admin(), &PersonId("p1".into())).await.unwrap();
        assert_eq!(admin_profile.person.name, "Ada");
        assert_eq!(
            admin_profile
                .filmography
                .iter()
                .map(|e| e.title.id())
                .collect::<Vec<_>>(),
            ["m1", "m2", "s1"]
        );
        assert_eq!(admin_profile.filmography[0].display_title, "m1");
        assert_eq!(
            admin_profile.filmography[0].character,
            Some("Hero".to_owned())
        );

        // Member (PG-13 cap) keeps only the PG-13 movie; R and TV-MA drop out.
        let member_profile = svc.person(&member(), &PersonId("p1".into())).await.unwrap();
        assert_eq!(
            member_profile
                .filmography
                .iter()
                .map(|e| e.title.id())
                .collect::<Vec<_>>(),
            ["m1"]
        );

        // p2 only directed m1.
        let p2 = svc.person(&admin(), &PersonId("p2".into())).await.unwrap();
        assert_eq!(p2.filmography.len(), 1);
        assert_eq!(p2.filmography[0].role, CreditRole::Director);

        assert!(matches!(
            svc.person(&admin(), &PersonId("nope".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));

        // Genres (deduped, name-sorted by the mock), optionally scoped to one kind.
        let names = |genres: Vec<Genre>| {
            genres
                .iter()
                .map(|g| g.name.clone())
                .collect::<Vec<String>>()
        };
        assert_eq!(
            names(svc.genres(&admin(), None).await.unwrap()),
            ["Action", "Drama"]
        );
        assert_eq!(
            names(svc.genres(&admin(), Some(TitleKind::Series)).await.unwrap()),
            ["Action"]
        );

        // Genre filter: m2 has no version, so no list reaches it whoever asks.
        let action = "Action";
        let admin_action = svc
            .movies(&admin(), &genre_query(action), page())
            .await
            .unwrap();
        assert_eq!(
            admin_action
                .items
                .iter()
                .map(|m| m.id.0.as_str())
                .collect::<Vec<_>>(),
            ["m1"]
        );
        let member_action = svc
            .movies(&member(), &genre_query(action), page())
            .await
            .unwrap();
        assert_eq!(
            member_action
                .items
                .iter()
                .map(|m| m.id.0.as_str())
                .collect::<Vec<_>>(),
            ["m1"]
        );

        // Genre filter on series.
        let series_action = svc
            .series(&admin(), &genre_query(action), page())
            .await
            .unwrap();
        assert_eq!(
            series_action
                .items
                .iter()
                .map(|s| s.id.0.as_str())
                .collect::<Vec<_>>(),
            ["s2"],
            "s1 carries the genre too, but has no file and so sits in no library"
        );

        // Detail aggregates carry enrichment.
        let detail = svc.movie(&admin(), &MovieId("m1".into())).await.unwrap();
        assert_eq!(detail.genres.len(), 1);
        assert_eq!(detail.credits.len(), 2);
        let series_detail = svc
            .series_detail(&admin(), &SeriesId("s1".into()))
            .await
            .unwrap();
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
        users
            .set_library_access(&UserId("u1".into()), &[LibraryId("lib1".into())])
            .await
            .unwrap();
        users.grant(
            &UserId("admin".into()),
            &[LibraryId("lib1".into()), LibraryId("lib2".into())],
        );
        CatalogServiceImpl::new(catalog, users)
    }

    #[tokio::test]
    async fn a_random_pick_never_reaches_past_the_reader_library_access() {
        let svc = random_fixture().await;

        assert_eq!(
            svc.random(&admin(), &RandomScope::Movies, &query())
                .await
                .unwrap(),
            VersionId("uhd-lib2".into()),
            "this account holds both libraries, so the best copy wins outright"
        );
        assert_eq!(
            svc.random(&member(), &RandomScope::Movies, &query())
                .await
                .unwrap(),
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
        users
            .set_library_access(&UserId("u1".into()), &[LibraryId("lib1".into())])
            .await
            .unwrap();
        users.grant(&UserId("admin".into()), &[LibraryId("lib1".into())]);
        let svc = CatalogServiceImpl::new(catalog, users);

        assert!(matches!(
            svc.random(&member(), &RandomScope::Movies, &query()).await,
            Err(CatalogError::NotFound)
        ));
        assert!(
            svc.random(&admin(), &RandomScope::Movies, &query())
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn a_random_pick_with_nothing_playable_is_not_found() {
        let svc = seeded().await;

        assert!(matches!(
            svc.random(
                &member(),
                &RandomScope::Season(SeasonId("se1".into())),
                &query()
            )
            .await,
            Err(CatalogError::NotFound)
        ));
        assert!(matches!(
            svc.random(
                &member(),
                &RandomScope::Collection(CollectionId("nope".into())),
                &query()
            )
            .await,
            Err(CatalogError::NotFound)
        ));
    }

    #[tokio::test]
    async fn a_random_episode_stays_inside_the_scope_it_was_asked_for() {
        let svc = seeded().await;

        assert_eq!(
            svc.random(
                &admin(),
                &RandomScope::Season(SeasonId("se2".into())),
                &query()
            )
            .await
            .unwrap(),
            VersionId("ev2".into())
        );
        assert_eq!(
            svc.random(
                &admin(),
                &RandomScope::Series(SeriesId("s2".into())),
                &query()
            )
            .await
            .unwrap(),
            VersionId("ev2".into())
        );
        assert!(
            matches!(
                svc.random(
                    &admin(),
                    &RandomScope::Series(SeriesId("s1".into())),
                    &query()
                )
                .await,
                Err(CatalogError::NotFound)
            ),
            "s1 has an episode but no file behind it"
        );
    }
}

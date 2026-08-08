use std::collections::HashMap;

use domain::catalog::{
    ArtworkRef, Collection, CollectionDetail, CollectionId, CollectionUpdate, Episode, EpisodeCard,
    EpisodeId, FilmographyEntry, Movie, MovieDetail, MovieId, NewCollection, PersonProfile, Season,
    SeasonId, Series, SeriesDetail, SeriesId, TitleCard, TitleId, TitleListFilter, TitleListQuery,
    TitleRef, Version, VersionDetail, VersionId,
};
use domain::common::{Page, PageRequest};
use domain::error::CatalogError;
use domain::library::LibraryId;
use domain::metadata::{ArtworkKind, ContentRating, Genre, PersonId};
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
    async fn rating_cap(&self, caller: &Principal) -> Result<Option<ContentRating>, CatalogError> {
        Ok(self
            .users
            .get(&caller.user)
            .await?
            .and_then(|user| user.max_content_rating))
    }

    async fn access_set(&self, caller: &Principal) -> Result<Vec<LibraryId>, CatalogError> {
        Ok(self
            .users
            .list_library_access(&caller.user)
            .await?
            .into_iter()
            .map(|entry| entry.library)
            .collect())
    }

    async fn library_scope(
        &self,
        caller: &Principal,
        library: Option<&LibraryId>,
    ) -> Result<Option<Vec<LibraryId>>, CatalogError> {
        if acl::is_admin(caller) {
            return Ok(library.map(|lib| vec![lib.clone()]));
        }
        let access = self.access_set(caller).await?;
        let libraries = match library {
            Some(lib) if acl::can_access_library(&access, lib) => vec![lib.clone()],
            Some(_) => Vec::new(),
            None => access,
        };
        Ok(Some(libraries))
    }

    async fn collection_mosaic_art(
        &self,
        movies: &[MovieId],
    ) -> Result<Vec<ArtworkRef>, CatalogError> {
        let mut posters = Vec::new();
        for movie_id in movies.iter().take(MOSAIC_TILES) {
            let Some(movie) = self.catalog.get_movie(movie_id).await? else {
                continue;
            };
            if let Some(poster) = collection_poster(&movie) {
                posters.push(poster);
            }
        }
        Ok(posters)
    }

    async fn build_filter(
        &self,
        caller: &Principal,
        query: &TitleListQuery,
    ) -> Result<TitleListFilter, CatalogError> {
        let blocked_ratings = if acl::is_admin(caller) {
            Vec::new()
        } else {
            ContentRating::blocked_by(self.rating_cap(caller).await?.as_ref())
        };
        Ok(TitleListFilter {
            genres: query.genres.clone(),
            libraries: self.library_scope(caller, query.library.as_ref()).await?,
            blocked_ratings,
            sort: query.sort,
            order: query.order,
        })
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
        let mut movies = Vec::with_capacity(collection.movies.len());
        for movie_id in &collection.movies {
            if let Some(movie) = self.catalog.get_movie(movie_id).await? {
                movies.push(movie);
            }
        }
        if collection.artwork.is_empty() {
            collection.artwork = movies
                .iter()
                .take(MOSAIC_TILES)
                .filter_map(collection_poster)
                .collect();
        }
        Ok(CollectionDetail { collection, movies })
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
            movies: input.movies,
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
            movies: update.movies,
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
        if !acl::is_admin(caller) {
            let cap = self.rating_cap(caller).await?;
            if !acl::rating_permits(cap.as_ref(), detail.movie.content_rating.as_ref()) {
                return Err(CatalogError::NotFound);
            }
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
        if !acl::is_admin(caller) {
            let cap = self.rating_cap(caller).await?;
            if !acl::rating_permits(cap.as_ref(), detail.series.content_rating.as_ref()) {
                return Err(CatalogError::NotFound);
            }
        }
        Ok(detail)
    }

    async fn seasons(
        &self,
        _caller: &Principal,
        series: &SeriesId,
    ) -> Result<Vec<Season>, CatalogError> {
        Ok(self.catalog.list_seasons(series).await?)
    }

    async fn season(&self, _caller: &Principal, id: &SeasonId) -> Result<Season, CatalogError> {
        self.catalog
            .get_season(id)
            .await?
            .ok_or(CatalogError::NotFound)
    }

    async fn episodes(
        &self,
        _caller: &Principal,
        season: &SeasonId,
    ) -> Result<Vec<Episode>, CatalogError> {
        Ok(self.catalog.list_episodes(season).await?)
    }

    async fn episode(&self, _caller: &Principal, id: &EpisodeId) -> Result<Episode, CatalogError> {
        self.catalog
            .get_episode(id)
            .await?
            .ok_or(CatalogError::NotFound)
    }

    async fn versions(
        &self,
        caller: &Principal,
        title: &TitleId,
        page: PageRequest,
    ) -> Result<Page<Version>, CatalogError> {
        if acl::is_admin(caller) {
            return Ok(self.catalog.list_versions(title, page).await?);
        }
        let access = self.access_set(caller).await?;
        let filtered: Vec<Version> = self
            .catalog
            .list_versions(title, PageRequest::ALL)
            .await?
            .items
            .into_iter()
            .filter(|version| acl::can_access_library(&access, &version.library))
            .collect();
        Ok(paginate(&filtered, page))
    }

    async fn library_versions(
        &self,
        caller: &Principal,
        library: &LibraryId,
        page: PageRequest,
    ) -> Result<Page<Version>, CatalogError> {
        if !acl::is_admin(caller) {
            let access = self.access_set(caller).await?;
            if !acl::can_access_library(&access, library) {
                return Ok(paginate(&Vec::<Version>::new(), page));
            }
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
        Ok(self.catalog.list_all_versions(page).await?)
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
        if !acl::is_admin(caller) {
            let access = self.access_set(caller).await?;
            if !acl::can_access_library(&access, &detail.version.library) {
                return Err(CatalogError::NotFound);
            }
        }
        Ok(detail)
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
        let admin = acl::is_admin(caller);
        let cap = if admin {
            None
        } else {
            self.rating_cap(caller).await?
        };
        let mut filmography = Vec::new();
        for credit in self.catalog.filmography(id).await? {
            let resolved = match &credit.title {
                TitleRef::Movie(movie) => self
                    .catalog
                    .get_movie(movie)
                    .await?
                    .map(|m| (m.title, m.year, m.artwork, m.content_rating)),
                TitleRef::Series(series) => self
                    .catalog
                    .get_series(series)
                    .await?
                    .map(|s| (s.title, s.year, s.artwork, s.content_rating)),
            };
            let Some((display_title, year, artwork, content_rating)) = resolved else {
                continue;
            };
            if !admin && !acl::rating_permits(cap.as_ref(), content_rating.as_ref()) {
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

    async fn genres(&self, _caller: &Principal) -> Result<Vec<Genre>, CatalogError> {
        Ok(self.catalog.list_genres().await?)
    }

    async fn title_cards(
        &self,
        caller: &Principal,
        ids: &[TitleId],
    ) -> Result<Vec<TitleCard>, CatalogError> {
        let admin = acl::is_admin(caller);
        let cap = if admin {
            None
        } else {
            self.rating_cap(caller).await?
        };
        let mut seasons: HashMap<SeasonId, Option<Season>> = HashMap::new();
        let mut series: HashMap<SeriesId, Option<Series>> = HashMap::new();
        let mut cards = Vec::new();
        for id in ids {
            match id {
                TitleId::Movie(movie_id) => {
                    let Some(movie) = self.catalog.get_movie(movie_id).await? else {
                        continue;
                    };
                    if !admin && !acl::rating_permits(cap.as_ref(), movie.content_rating.as_ref()) {
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
                    let series_title = match &series_id {
                        Some(sid) => {
                            let resolved = match series.get(sid).cloned() {
                                Some(cached) => cached,
                                None => {
                                    let loaded = self.catalog.get_series(sid).await?;
                                    series.insert(sid.clone(), loaded.clone());
                                    loaded
                                }
                            };
                            resolved.map(|s| s.title)
                        }
                        None => None,
                    };
                    cards.push(TitleCard::Episode(EpisodeCard {
                        season_number: season.as_ref().map(|s| s.number),
                        series: series_id,
                        series_title,
                        episode,
                    }));
                }
            }
        }
        Ok(cards)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{MockCatalogRepo, MockUserRepo};
    use domain::catalog::{ArtworkId, ArtworkOwner, SortOrder, TitleSort};
    use domain::common::Quality;
    use domain::metadata::GenreId;
    use domain::metadata::{Credit, CreditRole, Person, TitleEnrichment};
    use domain::user::{Role, User, UserId};
    use jiff::Timestamp;

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
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: code.map(rating),
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
            edition: None,
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
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    async fn seeded() -> CatalogServiceImpl<MockCatalogRepo, MockUserRepo> {
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(movie("m1", Some("PG-13")));
        catalog.add_movie(movie("m2", Some("R")));
        catalog.add_series(Series {
            id: SeriesId("s1".into()),
            title: "s1".into(),
            year: None,
            overview: None,
            content_rating: Some(ContentRating {
                system: "US-TV".into(),
                code: "TV-MA".into(),
            }),
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog.add_series(Series {
            id: SeriesId("s2".into()),
            title: "s2".into(),
            year: None,
            overview: None,
            content_rating: None,
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
            edition: None,
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

        let users = MockUserRepo::new();
        users.insert(capped_user());
        users
            .set_library_access(&UserId("u1".into()), &[LibraryId("lib1".into())])
            .await
            .unwrap();
        CatalogServiceImpl::new(catalog, users)
    }

    #[tokio::test]
    async fn admin_bypasses_rating_and_library_gates() {
        let svc = seeded().await;
        assert_eq!(
            svc.movies(&admin(), &query(), page()).await.unwrap().total,
            2
        );
        assert!(svc.movie(&admin(), &MovieId("m2".into())).await.is_ok());
        assert_eq!(
            svc.series(&admin(), &query(), page()).await.unwrap().total,
            2
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
    async fn all_versions_is_admin_only() {
        let svc = seeded().await;
        let all = svc.all_versions(&admin(), page()).await.unwrap();
        assert_eq!(all.total, 3);
        assert!(matches!(
            svc.all_versions(&member(), page()).await.unwrap_err(),
            CatalogError::Forbidden
        ));
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
            ["m2", "m1"]
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
            svc.seasons(&member(), &SeriesId("s1".into()))
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(svc.season(&member(), &SeasonId("se1".into())).await.is_ok());
        assert!(matches!(
            svc.season(&member(), &SeasonId("x".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));
        assert_eq!(
            svc.episodes(&member(), &SeasonId("se1".into()))
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(
            svc.episode(&member(), &EpisodeId("e1".into()))
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

    #[tokio::test]
    async fn backend_errors_propagate() {
        let catalog = MockCatalogRepo::new();
        catalog.set_fail();
        let svc = CatalogServiceImpl::new(catalog, MockUserRepo::new());
        assert!(matches!(
            svc.movies(&admin(), &query(), page()).await.unwrap_err(),
            CatalogError::Repository(_)
        ));
        assert!(matches!(
            svc.collections(&admin(), page()).await.unwrap_err(),
            CatalogError::Repository(_)
        ));
        assert!(matches!(
            svc.title_cards(&admin(), &[TitleId::Movie(MovieId("m1".into()))])
                .await
                .unwrap_err(),
            CatalogError::Repository(_)
        ));
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
                        widths: vec![180],
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
        assert_eq!(card_ids(&member_cards), ["m1", "e1"]);

        assert!(svc.title_cards(&member(), &[]).await.unwrap().is_empty());
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

        // Genres (deduped, name-sorted by the mock).
        let genres = svc.genres(&admin()).await.unwrap();
        assert_eq!(
            genres.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(),
            ["Action", "Drama"]
        );

        // Genre filter: admin sees both action movies; the member loses the R one.
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
            ["m1", "m2"]
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
            ["s1"]
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
}

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use domain::catalog::{
    Collection, CollectionId, CollectionUpdate, Episode, EpisodeId, FilmographyEntry, Movie,
    MovieDetail, MovieId, NewCollection, PersonProfile, Season, SeasonId, Series, SeriesDetail,
    SeriesId, TitleCard, TitleId, TitleKind, TitleListQuery, Version, VersionDetail, VersionId,
    sort_titles,
};
use domain::common::{Page, PageRequest};
use domain::error::CatalogError;
use domain::library::LibraryId;
use domain::media::{
    AudioTrack, Chapter, DetectedMarkers, EmbeddedSubtitleTrack, TrickplayAsset, VideoTrack,
};
use domain::metadata::{
    CreditedPerson, ExternalId, Extra, Genre, Person, PersonId, Rating, Studio,
};
use domain::service::CatalogService;
use domain::user::Principal;

use crate::page::paginate;

#[derive(Debug, Clone, Default)]
struct VersionExtras {
    video: Vec<VideoTrack>,
    audio: Vec<AudioTrack>,
    subtitles: Vec<EmbeddedSubtitleTrack>,
    chapters: Vec<Chapter>,
    markers: DetectedMarkers,
    trickplay: Vec<TrickplayAsset>,
}

#[derive(Debug, Clone, Default)]
struct DetailParts {
    genres: Vec<Genre>,
    credits: Vec<CreditedPerson>,
    studios: Vec<Studio>,
    ratings: Vec<Rating>,
    external_ids: Vec<ExternalId>,
    extras: Vec<Extra>,
}

#[derive(Debug, Default)]
struct State {
    movies: Vec<Movie>,
    series: Vec<Series>,
    seasons: Vec<Season>,
    episodes: Vec<Episode>,
    collections: Vec<Collection>,
    collection_seq: u64,
    versions: Vec<Version>,
    details: HashMap<VersionId, VersionExtras>,
    movie_details: HashMap<MovieId, DetailParts>,
    series_details: HashMap<SeriesId, DetailParts>,
    people: Vec<Person>,
    filmography: HashMap<PersonId, Vec<FilmographyEntry>>,
}

impl State {
    fn genre_catalog(&self) -> Vec<Genre> {
        let mut genres: Vec<Genre> = Vec::new();
        let details = self
            .movie_details
            .values()
            .chain(self.series_details.values());
        for parts in details {
            for genre in &parts.genres {
                if !genres.iter().any(|existing| existing.id == genre.id) {
                    genres.push(genre.clone());
                }
            }
        }
        genres.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.0.cmp(&b.id.0)));
        genres
    }

    fn titles_in_library(&self, kind: TitleKind, library: &LibraryId) -> HashSet<String> {
        let mut ids = HashSet::new();
        for version in self.versions.iter().filter(|v| &v.library == library) {
            match (kind, &version.title) {
                (TitleKind::Movie, TitleId::Movie(id)) => {
                    ids.insert(id.0.clone());
                }
                (TitleKind::Series, TitleId::Episode(episode)) => {
                    if let Some(series) = self
                        .episodes
                        .iter()
                        .find(|e| e.id == *episode)
                        .and_then(|e| self.seasons.iter().find(|s| s.id == e.season))
                        .map(|s| s.series.0.clone())
                    {
                        ids.insert(series);
                    }
                }
                _ => {}
            }
        }
        ids
    }
}

#[derive(Clone, Default)]
pub struct MockCatalogService {
    state: Arc<Mutex<State>>,
}

impl MockCatalogService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_movie(&self, movie: Movie) {
        self.state.lock().unwrap().movies.push(movie);
    }

    pub fn add_series(&self, series: Series) {
        self.state.lock().unwrap().series.push(series);
    }

    pub fn add_season(&self, season: Season) {
        self.state.lock().unwrap().seasons.push(season);
    }

    pub fn add_episode(&self, episode: Episode) {
        self.state.lock().unwrap().episodes.push(episode);
    }

    pub fn add_collection(&self, collection: Collection) {
        self.state.lock().unwrap().collections.push(collection);
    }

    pub fn add_version(&self, version: Version) {
        self.state.lock().unwrap().versions.push(version);
    }

    pub fn seed_version_detail(&self, detail: VersionDetail) {
        let mut state = self.state.lock().unwrap();
        state.details.insert(
            detail.version.id.clone(),
            VersionExtras {
                video: detail.video,
                audio: detail.audio,
                subtitles: detail.subtitles,
                chapters: detail.chapters,
                markers: detail.markers,
                trickplay: detail.trickplay,
            },
        );
        if !state.versions.iter().any(|v| v.id == detail.version.id) {
            state.versions.push(detail.version);
        }
    }

    pub fn seed_movie_detail(&self, detail: MovieDetail) {
        let mut state = self.state.lock().unwrap();
        let id = detail.movie.id.clone();
        if !state.movies.iter().any(|m| m.id == id) {
            state.movies.push(detail.movie);
        }
        state.movie_details.insert(
            id,
            DetailParts {
                genres: detail.genres,
                credits: detail.credits,
                studios: detail.studios,
                ratings: detail.ratings,
                external_ids: detail.external_ids,
                extras: detail.extras,
            },
        );
    }

    pub fn seed_series_detail(&self, detail: SeriesDetail) {
        let mut state = self.state.lock().unwrap();
        let id = detail.series.id.clone();
        if !state.series.iter().any(|s| s.id == id) {
            state.series.push(detail.series);
        }
        state.series_details.insert(
            id,
            DetailParts {
                genres: detail.genres,
                credits: detail.credits,
                studios: detail.studios,
                ratings: detail.ratings,
                external_ids: detail.external_ids,
                extras: detail.extras,
            },
        );
    }

    pub fn add_person(&self, person: Person) {
        self.state.lock().unwrap().people.push(person);
    }

    pub fn seed_filmography(&self, person: &PersonId, entries: Vec<FilmographyEntry>) {
        self.state
            .lock()
            .unwrap()
            .filmography
            .insert(person.clone(), entries);
    }
}

impl CatalogService for MockCatalogService {
    async fn collections(
        &self,
        _caller: &Principal,
        page: PageRequest,
    ) -> Result<Page<Collection>, CatalogError> {
        Ok(paginate(&self.state.lock().unwrap().collections, page))
    }

    async fn collection(
        &self,
        _caller: &Principal,
        id: &CollectionId,
    ) -> Result<Collection, CatalogError> {
        self.state
            .lock()
            .unwrap()
            .collections
            .iter()
            .find(|c| &c.id == id)
            .cloned()
            .ok_or(CatalogError::NotFound)
    }

    async fn create_collection(
        &self,
        _caller: &Principal,
        input: NewCollection,
    ) -> Result<Collection, CatalogError> {
        let mut state = self.state.lock().unwrap();
        state.collection_seq += 1;
        let collection = Collection {
            id: CollectionId(format!("col-{}", state.collection_seq)),
            name: input.name,
            overview: input.overview,
            movies: input.movies,
            artwork: Vec::new(),
        };
        state.collections.push(collection.clone());
        Ok(collection)
    }

    async fn update_collection(
        &self,
        _caller: &Principal,
        id: &CollectionId,
        update: CollectionUpdate,
    ) -> Result<Collection, CatalogError> {
        let mut state = self.state.lock().unwrap();
        let collection = state
            .collections
            .iter_mut()
            .find(|c| &c.id == id)
            .ok_or(CatalogError::NotFound)?;
        collection.name = update.name;
        collection.overview = update.overview;
        collection.movies = update.movies;
        let mut updated = collection.clone();
        updated.artwork = Vec::new();
        Ok(updated)
    }

    async fn delete_collection(
        &self,
        _caller: &Principal,
        id: &CollectionId,
    ) -> Result<(), CatalogError> {
        let mut state = self.state.lock().unwrap();
        let before = state.collections.len();
        state.collections.retain(|c| &c.id != id);
        if state.collections.len() == before {
            return Err(CatalogError::NotFound);
        }
        Ok(())
    }

    async fn movies(
        &self,
        _caller: &Principal,
        query: &TitleListQuery,
        page: PageRequest,
    ) -> Result<Page<Movie>, CatalogError> {
        let state = self.state.lock().unwrap();
        let allowed = query
            .library
            .as_ref()
            .map(|lib| state.titles_in_library(TitleKind::Movie, lib));
        let mut matched: Vec<Movie> = state
            .movies
            .iter()
            .filter(|movie| match &query.genre {
                Some(genre) => state
                    .movie_details
                    .get(&movie.id)
                    .is_some_and(|parts| parts.genres.iter().any(|g| &g.id == genre)),
                None => true,
            })
            .filter(|movie| allowed.as_ref().is_none_or(|ids| ids.contains(&movie.id.0)))
            .cloned()
            .collect();
        sort_titles(&mut matched, query.sort, query.order);
        Ok(paginate(&matched, page))
    }

    async fn movie(&self, _caller: &Principal, id: &MovieId) -> Result<MovieDetail, CatalogError> {
        let state = self.state.lock().unwrap();
        let movie = state
            .movies
            .iter()
            .find(|m| &m.id == id)
            .cloned()
            .ok_or(CatalogError::NotFound)?;
        let parts = state.movie_details.get(id).cloned().unwrap_or_default();
        Ok(MovieDetail {
            movie,
            genres: parts.genres,
            credits: parts.credits,
            studios: parts.studios,
            ratings: parts.ratings,
            external_ids: parts.external_ids,
            extras: parts.extras,
        })
    }

    async fn series(
        &self,
        _caller: &Principal,
        query: &TitleListQuery,
        page: PageRequest,
    ) -> Result<Page<Series>, CatalogError> {
        let state = self.state.lock().unwrap();
        let allowed = query
            .library
            .as_ref()
            .map(|lib| state.titles_in_library(TitleKind::Series, lib));
        let mut matched: Vec<Series> = state
            .series
            .iter()
            .filter(|series| match &query.genre {
                Some(genre) => state
                    .series_details
                    .get(&series.id)
                    .is_some_and(|parts| parts.genres.iter().any(|g| &g.id == genre)),
                None => true,
            })
            .filter(|series| {
                allowed
                    .as_ref()
                    .is_none_or(|ids| ids.contains(&series.id.0))
            })
            .cloned()
            .collect();
        sort_titles(&mut matched, query.sort, query.order);
        Ok(paginate(&matched, page))
    }

    async fn series_detail(
        &self,
        _caller: &Principal,
        id: &SeriesId,
    ) -> Result<SeriesDetail, CatalogError> {
        let state = self.state.lock().unwrap();
        let series = state
            .series
            .iter()
            .find(|s| &s.id == id)
            .cloned()
            .ok_or(CatalogError::NotFound)?;
        let parts = state.series_details.get(id).cloned().unwrap_or_default();
        Ok(SeriesDetail {
            series,
            genres: parts.genres,
            credits: parts.credits,
            studios: parts.studios,
            ratings: parts.ratings,
            external_ids: parts.external_ids,
            extras: parts.extras,
        })
    }

    async fn seasons(
        &self,
        _caller: &Principal,
        series: &SeriesId,
    ) -> Result<Vec<Season>, CatalogError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .seasons
            .iter()
            .filter(|s| &s.series == series)
            .cloned()
            .collect())
    }

    async fn season(&self, _caller: &Principal, id: &SeasonId) -> Result<Season, CatalogError> {
        self.state
            .lock()
            .unwrap()
            .seasons
            .iter()
            .find(|s| &s.id == id)
            .cloned()
            .ok_or(CatalogError::NotFound)
    }

    async fn episodes(
        &self,
        _caller: &Principal,
        season: &SeasonId,
    ) -> Result<Vec<Episode>, CatalogError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .episodes
            .iter()
            .filter(|e| &e.season == season)
            .cloned()
            .collect())
    }

    async fn episode(&self, _caller: &Principal, id: &EpisodeId) -> Result<Episode, CatalogError> {
        self.state
            .lock()
            .unwrap()
            .episodes
            .iter()
            .find(|e| &e.id == id)
            .cloned()
            .ok_or(CatalogError::NotFound)
    }

    async fn versions(
        &self,
        _caller: &Principal,
        title: &TitleId,
        page: PageRequest,
    ) -> Result<Page<Version>, CatalogError> {
        let matched: Vec<Version> = self
            .state
            .lock()
            .unwrap()
            .versions
            .iter()
            .filter(|v| &v.title == title)
            .cloned()
            .collect();
        Ok(paginate(&matched, page))
    }

    async fn library_versions(
        &self,
        _caller: &Principal,
        library: &LibraryId,
        page: PageRequest,
    ) -> Result<Page<Version>, CatalogError> {
        let matched: Vec<Version> = self
            .state
            .lock()
            .unwrap()
            .versions
            .iter()
            .filter(|v| &v.library == library)
            .cloned()
            .collect();
        Ok(paginate(&matched, page))
    }

    async fn version(
        &self,
        _caller: &Principal,
        id: &VersionId,
    ) -> Result<VersionDetail, CatalogError> {
        let state = self.state.lock().unwrap();
        let version = state
            .versions
            .iter()
            .find(|v| &v.id == id)
            .cloned()
            .ok_or(CatalogError::NotFound)?;
        let extras = state.details.get(id).cloned().unwrap_or_default();
        Ok(VersionDetail {
            version,
            video: extras.video,
            audio: extras.audio,
            subtitles: extras.subtitles,
            chapters: extras.chapters,
            markers: extras.markers,
            trickplay: extras.trickplay,
        })
    }

    async fn person(
        &self,
        _caller: &Principal,
        id: &PersonId,
    ) -> Result<PersonProfile, CatalogError> {
        let state = self.state.lock().unwrap();
        let person = state
            .people
            .iter()
            .find(|p| &p.id == id)
            .cloned()
            .ok_or(CatalogError::NotFound)?;
        let filmography = state.filmography.get(id).cloned().unwrap_or_default();
        Ok(PersonProfile {
            person,
            filmography,
        })
    }

    async fn genres(&self, _caller: &Principal) -> Result<Vec<Genre>, CatalogError> {
        Ok(self.state.lock().unwrap().genre_catalog())
    }

    async fn title_cards(
        &self,
        _caller: &Principal,
        ids: &[TitleId],
    ) -> Result<Vec<TitleCard>, CatalogError> {
        let state = self.state.lock().unwrap();
        let mut cards = Vec::new();
        for id in ids {
            match id {
                TitleId::Movie(movie_id) => {
                    if let Some(movie) = state.movies.iter().find(|m| &m.id == movie_id) {
                        cards.push(TitleCard::Movie(movie.clone()));
                    }
                }
                TitleId::Episode(episode_id) => {
                    if let Some(episode) = state.episodes.iter().find(|e| &e.id == episode_id) {
                        cards.push(TitleCard::Episode(episode.clone()));
                    }
                }
            }
        }
        Ok(cards)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{SortOrder, TitleSort};
    use domain::common::Quality;
    use domain::metadata::GenreId;
    use domain::user::{Role, UserId};
    use jiff::Timestamp;

    fn principal() -> Principal {
        Principal {
            user: UserId("u1".into()),
            role: Role::Admin,
        }
    }

    fn movie(id: &str) -> Movie {
        Movie {
            id: MovieId(id.to_string()),
            title: format!("Movie {id}"),
            year: Some(2020),
            overview: None,
            runtime_minutes: Some(100),
            content_rating: None,
            added_at: Timestamp::now(),
            artwork: Vec::new(),
        }
    }

    fn series(id: &str) -> Series {
        Series {
            id: SeriesId(id.to_string()),
            title: format!("Series {id}"),
            year: Some(2019),
            overview: None,
            content_rating: None,
            added_at: Timestamp::now(),
            artwork: Vec::new(),
        }
    }

    fn season(id: &str, series: &str) -> Season {
        Season {
            id: SeasonId(id.to_string()),
            series: SeriesId(series.to_string()),
            number: 1,
            title: None,
            overview: None,
            artwork: Vec::new(),
        }
    }

    fn episode(id: &str, season: &str) -> Episode {
        Episode {
            id: EpisodeId(id.to_string()),
            season: SeasonId(season.to_string()),
            number: 1,
            title: format!("Episode {id}"),
            overview: None,
            runtime_minutes: Some(42),
            air_date: None,
            added_at: Timestamp::now(),
            artwork: Vec::new(),
        }
    }

    fn version(id: &str, movie: &str) -> Version {
        Version {
            id: VersionId(id.to_string()),
            title: TitleId::Movie(MovieId(movie.to_string())),
            library: LibraryId("lib1".to_string()),
            quality: Quality::Hd,
            container: "mkv".to_string(),
            path: format!("/media/{id}.mkv"),
            size_bytes: 1024,
            duration_ms: 1000,
            edition: None,
            available: true,
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

    fn genre_query(id: &str) -> TitleListQuery {
        TitleListQuery {
            genre: Some(GenreId(id.into())),
            ..TitleListQuery::default()
        }
    }

    #[tokio::test]
    async fn movies_list_and_detail() {
        let svc = MockCatalogService::new();
        svc.add_movie(movie("m1"));
        svc.add_movie(movie("m2"));

        let page = svc.movies(&principal(), &query(), page()).await.unwrap();
        assert_eq!(page.total, 2);
        assert_eq!(page.items.len(), 2);

        let found = svc
            .movie(&principal(), &MovieId("m1".into()))
            .await
            .unwrap();
        assert_eq!(found.movie.id, MovieId("m1".into()));
    }

    #[tokio::test]
    async fn movies_sorted_and_library_scoped() {
        let svc = MockCatalogService::new();
        svc.add_movie(movie("m1"));
        svc.add_movie(movie("m2"));
        svc.add_movie(movie("m3"));

        let desc = TitleListQuery {
            sort: TitleSort::Title,
            order: SortOrder::Desc,
            ..TitleListQuery::default()
        };
        let sorted = svc.movies(&principal(), &desc, page()).await.unwrap();
        assert_eq!(
            sorted
                .items
                .iter()
                .map(|m| m.id.0.as_str())
                .collect::<Vec<_>>(),
            ["m3", "m2", "m1"]
        );

        svc.add_version(version("v1", "m1"));
        svc.add_version(Version {
            library: LibraryId("lib2".into()),
            ..version("v2", "m2")
        });
        let scoped = TitleListQuery {
            library: Some(LibraryId("lib1".into())),
            ..TitleListQuery::default()
        };
        let in_lib1 = svc.movies(&principal(), &scoped, page()).await.unwrap();
        assert_eq!(
            in_lib1
                .items
                .iter()
                .map(|m| m.id.0.as_str())
                .collect::<Vec<_>>(),
            ["m1"]
        );

        let unknown = TitleListQuery {
            library: Some(LibraryId("ghost".into())),
            ..TitleListQuery::default()
        };
        assert!(
            svc.movies(&principal(), &unknown, page())
                .await
                .unwrap()
                .items
                .is_empty()
        );
    }

    #[tokio::test]
    async fn series_library_scoped_via_episode_chain() {
        let svc = MockCatalogService::new();
        svc.add_series(series("s1"));
        svc.add_series(series("s2"));
        svc.add_season(season("se1", "s1"));
        svc.add_episode(episode("e1", "se1"));
        svc.add_version(Version {
            title: TitleId::Episode(EpisodeId("e1".into())),
            library: LibraryId("lib1".into()),
            ..version("ev1", "unused")
        });

        let scoped = TitleListQuery {
            library: Some(LibraryId("lib1".into())),
            ..TitleListQuery::default()
        };
        let in_lib1 = svc.series(&principal(), &scoped, page()).await.unwrap();
        assert_eq!(
            in_lib1
                .items
                .iter()
                .map(|s| s.id.0.as_str())
                .collect::<Vec<_>>(),
            ["s1"]
        );
    }

    #[tokio::test]
    async fn title_cards_resolve_in_order_and_skip_missing() {
        let svc = MockCatalogService::new();
        svc.add_movie(movie("m1"));
        svc.add_episode(episode("e1", "se1"));

        let ids = vec![
            TitleId::Episode(EpisodeId("e1".into())),
            TitleId::Movie(MovieId("ghost".into())),
            TitleId::Movie(MovieId("m1".into())),
        ];
        let cards = svc.title_cards(&principal(), &ids).await.unwrap();
        let got: Vec<&str> = cards
            .iter()
            .map(|card| match card {
                TitleCard::Movie(m) => m.id.0.as_str(),
                TitleCard::Episode(e) => e.id.0.as_str(),
            })
            .collect();
        assert_eq!(got, ["e1", "m1"]);
        assert!(svc.title_cards(&principal(), &[]).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn movie_missing_is_not_found() {
        let svc = MockCatalogService::new();
        let err = svc
            .movie(&principal(), &MovieId("nope".into()))
            .await
            .unwrap_err();
        assert!(matches!(err, CatalogError::NotFound));
    }

    #[tokio::test]
    async fn collections_list_and_detail() {
        let svc = MockCatalogService::new();
        svc.add_collection(Collection {
            id: CollectionId("c1".into()),
            name: "Trilogy".into(),
            overview: None,
            movies: vec![MovieId("m1".into())],
            artwork: Vec::new(),
        });

        let page = svc.collections(&principal(), page()).await.unwrap();
        assert_eq!(page.total, 1);
        assert!(
            svc.collection(&principal(), &CollectionId("c1".into()))
                .await
                .is_ok()
        );
        assert!(matches!(
            svc.collection(&principal(), &CollectionId("x".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));
    }

    #[tokio::test]
    async fn series_seasons_episodes() {
        let svc = MockCatalogService::new();
        svc.add_series(series("s1"));
        svc.add_season(season("se1", "s1"));
        svc.add_season(season("se2", "other"));
        svc.add_episode(episode("e1", "se1"));

        let page = svc.series(&principal(), &query(), page()).await.unwrap();
        assert_eq!(page.total, 1);
        assert!(
            svc.series_detail(&principal(), &SeriesId("s1".into()))
                .await
                .is_ok()
        );
        assert!(matches!(
            svc.series_detail(&principal(), &SeriesId("x".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));

        let seasons = svc
            .seasons(&principal(), &SeriesId("s1".into()))
            .await
            .unwrap();
        assert_eq!(seasons.len(), 1);

        let episodes = svc
            .episodes(&principal(), &SeasonId("se1".into()))
            .await
            .unwrap();
        assert_eq!(episodes.len(), 1);
        assert!(
            svc.episode(&principal(), &EpisodeId("e1".into()))
                .await
                .is_ok()
        );
        assert!(matches!(
            svc.episode(&principal(), &EpisodeId("x".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));
    }

    #[tokio::test]
    async fn versions_filtered_by_title() {
        let svc = MockCatalogService::new();
        svc.add_version(version("v1", "m1"));
        svc.add_version(version("v2", "m2"));

        let versions = svc
            .versions(&principal(), &TitleId::Movie(MovieId("m1".into())), page())
            .await
            .unwrap();
        assert_eq!(versions.total, 1);
        assert_eq!(versions.items.len(), 1);
    }

    #[tokio::test]
    async fn version_detail_by_id() {
        let svc = MockCatalogService::new();
        svc.add_version(version("v1", "m1"));

        let detail = svc
            .version(&principal(), &VersionId("v1".into()))
            .await
            .unwrap();
        assert_eq!(detail.version.id, VersionId("v1".into()));
        assert!(detail.video.is_empty());
        assert!(matches!(
            svc.version(&principal(), &VersionId("nope".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));
    }

    #[tokio::test]
    async fn seeded_version_detail_hydrates_tracks() {
        use domain::media::{IntroMarker, TrickplayAsset, VideoTrack};

        let svc = MockCatalogService::new();
        let vid = VersionId("v1".into());
        svc.seed_version_detail(VersionDetail {
            version: version("v1", "m1"),
            video: vec![VideoTrack {
                index: 0,
                codec: "hevc".into(),
                width: 1920,
                height: 1080,
                bit_depth: 8,
                hdr: None,
                frame_rate: 24.0,
                bitrate: Some(8_000_000),
            }],
            audio: Vec::new(),
            subtitles: Vec::new(),
            chapters: Vec::new(),
            markers: DetectedMarkers {
                intros: vec![IntroMarker {
                    version: vid.clone(),
                    start_ms: 0,
                    end_ms: 30_000,
                }],
                credits: Vec::new(),
            },
            trickplay: vec![TrickplayAsset {
                version: vid.clone(),
                interval_ms: 10_000,
                columns: 4,
                rows: 4,
                tile_width: 320,
                tile_height: 180,
                sheet_paths: vec!["sheet-000.jpg".into()],
            }],
        });

        let detail = svc.version(&principal(), &vid).await.unwrap();
        assert_eq!(detail.video.len(), 1);
        assert_eq!(detail.markers.intros.len(), 1);
        assert_eq!(detail.trickplay.len(), 1);
    }

    #[tokio::test]
    async fn season_detail() {
        let svc = MockCatalogService::new();
        svc.add_season(season("se1", "s1"));
        assert!(
            svc.season(&principal(), &SeasonId("se1".into()))
                .await
                .is_ok()
        );
        assert!(matches!(
            svc.season(&principal(), &SeasonId("x".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));
    }

    #[tokio::test]
    async fn collection_create_update_delete() {
        let svc = MockCatalogService::new();
        let created = svc
            .create_collection(
                &principal(),
                NewCollection {
                    name: "Saga".into(),
                    overview: Some("epic".into()),
                    movies: vec![MovieId("m1".into())],
                },
            )
            .await
            .unwrap();
        assert_eq!(created.name, "Saga");
        assert_eq!(
            svc.collection(&principal(), &created.id)
                .await
                .unwrap()
                .movies
                .len(),
            1
        );

        let updated = svc
            .update_collection(
                &principal(),
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
        assert!(updated.movies.is_empty());
        assert!(matches!(
            svc.update_collection(
                &principal(),
                &CollectionId("nope".into()),
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

        svc.delete_collection(&principal(), &created.id)
            .await
            .unwrap();
        assert!(matches!(
            svc.delete_collection(&principal(), &created.id)
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));
    }

    #[tokio::test]
    async fn library_versions_filtered() {
        let svc = MockCatalogService::new();
        for (vid, lib) in [("v1", "lib1"), ("v2", "lib2")] {
            svc.add_version(Version {
                id: VersionId(vid.into()),
                title: TitleId::Movie(MovieId("m1".into())),
                library: LibraryId(lib.into()),
                quality: Quality::Hd,
                container: "mkv".into(),
                path: format!("/media/{vid}.mkv"),
                size_bytes: 1,
                duration_ms: 1,
                edition: None,
                available: true,
            });
        }
        let from_lib1 = svc
            .library_versions(&principal(), &LibraryId("lib1".into()), page())
            .await
            .unwrap();
        assert_eq!(from_lib1.total, 1);
        assert_eq!(from_lib1.items.len(), 1);
    }

    #[tokio::test]
    async fn detail_person_and_genre_surface() {
        use domain::catalog::TitleRef;
        use domain::metadata::{CreditRole, ExtraKind, StudioId};

        let svc = MockCatalogService::new();
        svc.seed_movie_detail(MovieDetail {
            movie: movie("m1"),
            genres: vec![Genre {
                id: GenreId("g1".into()),
                name: "Action".into(),
            }],
            credits: vec![CreditedPerson {
                person: Person {
                    id: PersonId("p1".into()),
                    name: "Ada".into(),
                },
                role: CreditRole::Actor,
                character: Some("Hero".into()),
                order: 0,
            }],
            studios: vec![Studio {
                id: StudioId("st1".into()),
                name: "Acme".into(),
            }],
            ratings: vec![Rating {
                source: "tmdb".into(),
                value: 8.5,
            }],
            external_ids: vec![ExternalId {
                source: "tmdb".into(),
                value: "603".into(),
            }],
            extras: vec![Extra {
                kind: ExtraKind::Trailer,
                title: "Teaser".into(),
                path: "/x.mkv".into(),
            }],
        });
        svc.seed_movie_detail(MovieDetail {
            movie: movie("m2"),
            genres: vec![Genre {
                id: GenreId("g2".into()),
                name: "Drama".into(),
            }],
            credits: Vec::new(),
            studios: Vec::new(),
            ratings: Vec::new(),
            external_ids: Vec::new(),
            extras: Vec::new(),
        });
        svc.seed_series_detail(SeriesDetail {
            series: series("s1"),
            genres: vec![Genre {
                id: GenreId("g1".into()),
                name: "Action".into(),
            }],
            credits: Vec::new(),
            studios: Vec::new(),
            ratings: Vec::new(),
            external_ids: Vec::new(),
            extras: Vec::new(),
        });
        svc.add_person(Person {
            id: PersonId("p1".into()),
            name: "Ada".into(),
        });
        svc.seed_filmography(
            &PersonId("p1".into()),
            vec![FilmographyEntry {
                title: TitleRef::Movie(MovieId("m1".into())),
                display_title: "Movie m1".into(),
                year: Some(2020),
                artwork: Vec::new(),
                role: CreditRole::Actor,
                character: Some("Hero".into()),
            }],
        );

        let detail = svc
            .movie(&principal(), &MovieId("m1".into()))
            .await
            .unwrap();
        assert_eq!(detail.genres.len(), 1);
        assert_eq!(detail.credits[0].person.name, "Ada");
        assert_eq!(detail.studios[0].name, "Acme");
        assert_eq!(detail.ratings[0].value, 8.5);
        assert_eq!(detail.external_ids[0].value, "603");
        assert_eq!(detail.extras[0].title, "Teaser");

        // A movie with no seeded detail falls back to empty enrichment.
        svc.add_movie(movie("m3"));
        let bare = svc
            .movie(&principal(), &MovieId("m3".into()))
            .await
            .unwrap();
        assert!(bare.genres.is_empty());

        let series_detail = svc
            .series_detail(&principal(), &SeriesId("s1".into()))
            .await
            .unwrap();
        assert_eq!(series_detail.genres.len(), 1);

        let genres = svc.genres(&principal()).await.unwrap();
        assert_eq!(
            genres.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(),
            ["Action", "Drama"]
        );

        let action_movies = svc
            .movies(&principal(), &genre_query("g1"), page())
            .await
            .unwrap();
        assert_eq!(
            action_movies
                .items
                .iter()
                .map(|m| m.id.0.as_str())
                .collect::<Vec<_>>(),
            ["m1"]
        );
        let action_series = svc
            .series(&principal(), &genre_query("g1"), page())
            .await
            .unwrap();
        assert_eq!(action_series.total, 1);

        let profile = svc
            .person(&principal(), &PersonId("p1".into()))
            .await
            .unwrap();
        assert_eq!(profile.person.name, "Ada");
        assert_eq!(profile.filmography.len(), 1);
        assert_eq!(profile.filmography[0].display_title, "Movie m1");
        assert!(matches!(
            svc.person(&principal(), &PersonId("nope".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));
    }
}

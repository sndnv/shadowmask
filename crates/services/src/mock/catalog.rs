use std::sync::{Arc, Mutex};

use domain::catalog::{
    Collection, CollectionId, Episode, EpisodeId, Movie, MovieId, Season, SeasonId, Series,
    SeriesId, TitleId, Version,
};
use domain::common::{Page, PageRequest};
use domain::error::CatalogError;
use domain::service::CatalogService;

use crate::mock::page::paginate;

#[derive(Debug, Default)]
struct State {
    movies: Vec<Movie>,
    series: Vec<Series>,
    seasons: Vec<Season>,
    episodes: Vec<Episode>,
    collections: Vec<Collection>,
    versions: Vec<Version>,
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
}

impl CatalogService for MockCatalogService {
    async fn collections(&self, page: PageRequest) -> Result<Page<Collection>, CatalogError> {
        Ok(paginate(&self.state.lock().unwrap().collections, page))
    }

    async fn collection(&self, id: &CollectionId) -> Result<Collection, CatalogError> {
        self.state
            .lock()
            .unwrap()
            .collections
            .iter()
            .find(|c| &c.id == id)
            .cloned()
            .ok_or(CatalogError::NotFound)
    }

    async fn movies(&self, page: PageRequest) -> Result<Page<Movie>, CatalogError> {
        Ok(paginate(&self.state.lock().unwrap().movies, page))
    }

    async fn movie(&self, id: &MovieId) -> Result<Movie, CatalogError> {
        self.state
            .lock()
            .unwrap()
            .movies
            .iter()
            .find(|m| &m.id == id)
            .cloned()
            .ok_or(CatalogError::NotFound)
    }

    async fn series(&self, page: PageRequest) -> Result<Page<Series>, CatalogError> {
        Ok(paginate(&self.state.lock().unwrap().series, page))
    }

    async fn series_detail(&self, id: &SeriesId) -> Result<Series, CatalogError> {
        self.state
            .lock()
            .unwrap()
            .series
            .iter()
            .find(|s| &s.id == id)
            .cloned()
            .ok_or(CatalogError::NotFound)
    }

    async fn seasons(&self, series: &SeriesId) -> Result<Vec<Season>, CatalogError> {
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

    async fn episodes(&self, season: &SeasonId) -> Result<Vec<Episode>, CatalogError> {
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

    async fn episode(&self, id: &EpisodeId) -> Result<Episode, CatalogError> {
        self.state
            .lock()
            .unwrap()
            .episodes
            .iter()
            .find(|e| &e.id == id)
            .cloned()
            .ok_or(CatalogError::NotFound)
    }

    async fn versions(&self, title: &TitleId) -> Result<Vec<Version>, CatalogError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .versions
            .iter()
            .filter(|v| &v.title == title)
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::common::Quality;
    use jiff::Timestamp;

    fn movie(id: &str) -> Movie {
        Movie {
            id: MovieId(id.to_string()),
            title: format!("Movie {id}"),
            year: Some(2020),
            overview: None,
            runtime_minutes: Some(100),
            content_rating: None,
            added_at: Timestamp::now(),
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
        }
    }

    fn season(id: &str, series: &str) -> Season {
        Season {
            id: SeasonId(id.to_string()),
            series: SeriesId(series.to_string()),
            number: 1,
            title: None,
            overview: None,
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
        }
    }

    fn version(id: &str, movie: &str) -> Version {
        Version {
            id: domain::catalog::VersionId(id.to_string()),
            title: TitleId::Movie(MovieId(movie.to_string())),
            quality: Quality::Hd,
            container: "mkv".to_string(),
            path: format!("/media/{id}.mkv"),
            size_bytes: 1024,
        }
    }

    #[tokio::test]
    async fn movies_list_and_detail() {
        let svc = MockCatalogService::new();
        svc.add_movie(movie("m1"));
        svc.add_movie(movie("m2"));

        let page = svc
            .movies(PageRequest {
                offset: 0,
                limit: 10,
            })
            .await
            .unwrap();
        assert_eq!(page.total, 2);
        assert_eq!(page.items.len(), 2);

        let found = svc.movie(&MovieId("m1".into())).await.unwrap();
        assert_eq!(found.id, MovieId("m1".into()));
    }

    #[tokio::test]
    async fn movie_missing_is_not_found() {
        let svc = MockCatalogService::new();
        let err = svc.movie(&MovieId("nope".into())).await.unwrap_err();
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
        });

        let page = svc
            .collections(PageRequest {
                offset: 0,
                limit: 10,
            })
            .await
            .unwrap();
        assert_eq!(page.total, 1);
        assert!(svc.collection(&CollectionId("c1".into())).await.is_ok());
        assert!(matches!(
            svc.collection(&CollectionId("x".into())).await.unwrap_err(),
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

        let page = svc
            .series(PageRequest {
                offset: 0,
                limit: 10,
            })
            .await
            .unwrap();
        assert_eq!(page.total, 1);
        assert!(svc.series_detail(&SeriesId("s1".into())).await.is_ok());
        assert!(matches!(
            svc.series_detail(&SeriesId("x".into())).await.unwrap_err(),
            CatalogError::NotFound
        ));

        let seasons = svc.seasons(&SeriesId("s1".into())).await.unwrap();
        assert_eq!(seasons.len(), 1);

        let episodes = svc.episodes(&SeasonId("se1".into())).await.unwrap();
        assert_eq!(episodes.len(), 1);
        assert!(svc.episode(&EpisodeId("e1".into())).await.is_ok());
        assert!(matches!(
            svc.episode(&EpisodeId("x".into())).await.unwrap_err(),
            CatalogError::NotFound
        ));
    }

    #[tokio::test]
    async fn versions_filtered_by_title() {
        let svc = MockCatalogService::new();
        svc.add_version(version("v1", "m1"));
        svc.add_version(version("v2", "m2"));

        let versions = svc
            .versions(&TitleId::Movie(MovieId("m1".into())))
            .await
            .unwrap();
        assert_eq!(versions.len(), 1);
    }
}

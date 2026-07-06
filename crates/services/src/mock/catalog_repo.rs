use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use domain::catalog::{
    Collection, CollectionId, Episode, EpisodeId, Movie, MovieId, Season, SeasonId, Series,
    SeriesId, TitleId, Version, VersionDetail, VersionId,
};
use domain::common::{Page, PageRequest};
use domain::error::RepositoryError;
use domain::library::LibraryId;
use domain::media::DetectedMarkers;
use domain::repository::CatalogRepository;

use crate::page::paginate;

#[derive(Default)]
struct State {
    movies: Vec<Movie>,
    series: Vec<Series>,
    seasons: Vec<Season>,
    episodes: Vec<Episode>,
    collections: Vec<Collection>,
    versions: Vec<Version>,
}

#[derive(Clone, Default)]
pub struct MockCatalogRepo {
    state: Arc<Mutex<State>>,
    fail: Arc<AtomicBool>,
}

impl MockCatalogRepo {
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

    pub fn set_fail(&self) {
        self.fail.store(true, Ordering::Relaxed);
    }

    fn guard(&self) -> Result<(), RepositoryError> {
        if self.fail.load(Ordering::Relaxed) {
            Err(RepositoryError::Backend("mock catalog failure".to_owned()))
        } else {
            Ok(())
        }
    }
}

impl CatalogRepository for MockCatalogRepo {
    async fn list_movies(&self, page: PageRequest) -> Result<Page<Movie>, RepositoryError> {
        self.guard()?;
        Ok(paginate(&self.state.lock().unwrap().movies, page))
    }

    async fn get_movie(&self, id: &MovieId) -> Result<Option<Movie>, RepositoryError> {
        self.guard()?;
        Ok(self
            .state
            .lock()
            .unwrap()
            .movies
            .iter()
            .find(|m| &m.id == id)
            .cloned())
    }

    async fn list_series(&self, page: PageRequest) -> Result<Page<Series>, RepositoryError> {
        self.guard()?;
        Ok(paginate(&self.state.lock().unwrap().series, page))
    }

    async fn get_series(&self, id: &SeriesId) -> Result<Option<Series>, RepositoryError> {
        self.guard()?;
        Ok(self
            .state
            .lock()
            .unwrap()
            .series
            .iter()
            .find(|s| &s.id == id)
            .cloned())
    }

    async fn list_seasons(&self, series: &SeriesId) -> Result<Vec<Season>, RepositoryError> {
        self.guard()?;
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

    async fn list_episodes(&self, season: &SeasonId) -> Result<Vec<Episode>, RepositoryError> {
        self.guard()?;
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

    async fn get_episode(&self, id: &EpisodeId) -> Result<Option<Episode>, RepositoryError> {
        self.guard()?;
        Ok(self
            .state
            .lock()
            .unwrap()
            .episodes
            .iter()
            .find(|e| &e.id == id)
            .cloned())
    }

    async fn list_collections(
        &self,
        page: PageRequest,
    ) -> Result<Page<Collection>, RepositoryError> {
        self.guard()?;
        Ok(paginate(&self.state.lock().unwrap().collections, page))
    }

    async fn get_collection(
        &self,
        id: &CollectionId,
    ) -> Result<Option<Collection>, RepositoryError> {
        self.guard()?;
        Ok(self
            .state
            .lock()
            .unwrap()
            .collections
            .iter()
            .find(|c| &c.id == id)
            .cloned())
    }

    async fn upsert_collection(&self, collection: Collection) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if let Some(existing) = state.collections.iter_mut().find(|c| c.id == collection.id) {
            *existing = collection;
        } else {
            state.collections.push(collection);
        }
        Ok(())
    }

    async fn delete_collection(&self, id: &CollectionId) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state
            .lock()
            .unwrap()
            .collections
            .retain(|c| &c.id != id);
        Ok(())
    }

    async fn get_season(&self, id: &SeasonId) -> Result<Option<Season>, RepositoryError> {
        self.guard()?;
        Ok(self
            .state
            .lock()
            .unwrap()
            .seasons
            .iter()
            .find(|s| &s.id == id)
            .cloned())
    }

    async fn list_versions(
        &self,
        title: &TitleId,
        page: PageRequest,
    ) -> Result<Page<Version>, RepositoryError> {
        self.guard()?;
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

    async fn list_library_versions(
        &self,
        library: &LibraryId,
        page: PageRequest,
    ) -> Result<Page<Version>, RepositoryError> {
        self.guard()?;
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

    async fn version_detail(
        &self,
        id: &VersionId,
    ) -> Result<Option<VersionDetail>, RepositoryError> {
        self.guard()?;
        let version = self
            .state
            .lock()
            .unwrap()
            .versions
            .iter()
            .find(|v| &v.id == id)
            .cloned();
        Ok(version.map(|version| VersionDetail {
            version,
            video: Vec::new(),
            audio: Vec::new(),
            subtitles: Vec::new(),
            chapters: Vec::new(),
            markers: DetectedMarkers::default(),
            trickplay: Vec::new(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::common::Quality;
    use jiff::Timestamp;

    fn page() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 10,
        }
    }

    fn movie(id: &str) -> Movie {
        Movie {
            id: MovieId(id.to_owned()),
            title: id.to_owned(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            added_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn series(id: &str) -> Series {
        Series {
            id: SeriesId(id.to_owned()),
            title: id.to_owned(),
            year: None,
            overview: None,
            content_rating: None,
            added_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn season(id: &str, series: &str) -> Season {
        Season {
            id: SeasonId(id.to_owned()),
            series: SeriesId(series.to_owned()),
            number: 1,
            title: None,
            overview: None,
        }
    }

    fn episode(id: &str, season: &str) -> Episode {
        Episode {
            id: EpisodeId(id.to_owned()),
            season: SeasonId(season.to_owned()),
            number: 1,
            title: id.to_owned(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            added_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn version(id: &str) -> Version {
        Version {
            id: VersionId(id.to_owned()),
            title: TitleId::Movie(MovieId("m1".to_owned())),
            library: LibraryId("lib1".to_owned()),
            quality: Quality::Hd,
            container: "mkv".to_owned(),
            path: format!("/media/{id}.mkv"),
            size_bytes: 1,
            duration_ms: 1000,
            edition: None,
        }
    }

    #[tokio::test]
    async fn stores_and_reads_every_entity() {
        let repo = MockCatalogRepo::new();
        repo.add_movie(movie("m1"));
        repo.add_series(series("s1"));
        repo.add_season(season("se1", "s1"));
        repo.add_episode(episode("e1", "se1"));
        repo.add_collection(Collection {
            id: CollectionId("c1".to_owned()),
            name: "Saga".to_owned(),
            overview: None,
            movies: vec![MovieId("m1".to_owned())],
        });
        repo.add_version(version("v1"));

        assert_eq!(repo.list_movies(page()).await.unwrap().total, 1);
        assert!(
            repo.get_movie(&MovieId("m1".into()))
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            repo.get_movie(&MovieId("x".into()))
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(repo.list_series(page()).await.unwrap().total, 1);
        assert!(
            repo.get_series(&SeriesId("s1".into()))
                .await
                .unwrap()
                .is_some()
        );
        assert_eq!(
            repo.list_seasons(&SeriesId("s1".into()))
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(
            repo.get_season(&SeasonId("se1".into()))
                .await
                .unwrap()
                .is_some()
        );
        assert_eq!(
            repo.list_episodes(&SeasonId("se1".into()))
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(
            repo.get_episode(&EpisodeId("e1".into()))
                .await
                .unwrap()
                .is_some()
        );
        assert_eq!(repo.list_collections(page()).await.unwrap().total, 1);
        assert!(
            repo.get_collection(&CollectionId("c1".into()))
                .await
                .unwrap()
                .is_some()
        );
        assert_eq!(
            repo.list_versions(&TitleId::Movie(MovieId("m1".into())), page())
                .await
                .unwrap()
                .total,
            1
        );
        assert_eq!(
            repo.list_library_versions(&LibraryId("lib1".into()), page())
                .await
                .unwrap()
                .total,
            1
        );
        assert!(
            repo.version_detail(&VersionId("v1".into()))
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            repo.version_detail(&VersionId("x".into()))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn upsert_and_delete_collection() {
        let repo = MockCatalogRepo::new();
        repo.upsert_collection(Collection {
            id: CollectionId("c1".into()),
            name: "First".into(),
            overview: None,
            movies: Vec::new(),
        })
        .await
        .unwrap();
        repo.upsert_collection(Collection {
            id: CollectionId("c1".into()),
            name: "Renamed".into(),
            overview: None,
            movies: Vec::new(),
        })
        .await
        .unwrap();
        let stored = repo
            .get_collection(&CollectionId("c1".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.name, "Renamed");

        repo.delete_collection(&CollectionId("c1".into()))
            .await
            .unwrap();
        assert!(
            repo.get_collection(&CollectionId("c1".into()))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn surfaces_backend_failure() {
        let repo = MockCatalogRepo::new();
        repo.set_fail();
        assert!(repo.list_movies(page()).await.is_err());
        assert!(repo.get_movie(&MovieId("m1".into())).await.is_err());
        assert!(repo.list_series(page()).await.is_err());
        assert!(repo.get_series(&SeriesId("s1".into())).await.is_err());
        assert!(repo.list_seasons(&SeriesId("s1".into())).await.is_err());
        assert!(repo.list_episodes(&SeasonId("se1".into())).await.is_err());
        assert!(repo.get_episode(&EpisodeId("e1".into())).await.is_err());
        assert!(repo.list_collections(page()).await.is_err());
        assert!(
            repo.get_collection(&CollectionId("c1".into()))
                .await
                .is_err()
        );
        assert!(
            repo.upsert_collection(Collection {
                id: CollectionId("c1".into()),
                name: "x".into(),
                overview: None,
                movies: Vec::new(),
            })
            .await
            .is_err()
        );
        assert!(
            repo.delete_collection(&CollectionId("c1".into()))
                .await
                .is_err()
        );
        assert!(repo.get_season(&SeasonId("se1".into())).await.is_err());
        assert!(
            repo.list_versions(&TitleId::Movie(MovieId("m1".into())), page())
                .await
                .is_err()
        );
        assert!(
            repo.list_library_versions(&LibraryId("lib1".into()), page())
                .await
                .is_err()
        );
        assert!(repo.version_detail(&VersionId("v1".into())).await.is_err());
    }
}

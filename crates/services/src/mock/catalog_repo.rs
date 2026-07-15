use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use domain::catalog::{
    ArtworkOwner, ArtworkRef, Collection, CollectionId, Episode, EpisodeId, Movie, MovieDetail,
    MovieId, Season, SeasonId, Series, SeriesDetail, SeriesId, TitleId, TitleKind, TitleRef,
    Version, VersionDetail, VersionId,
};
use domain::common::{Page, PageRequest};
use domain::error::RepositoryError;
use domain::library::LibraryId;
use domain::media::{
    AudioTrack, Chapter, DetectedMarkers, EmbeddedSubtitleTrack, TrickplayAsset, VideoTrack,
};
use domain::metadata::{Credit, CreditedPerson, Genre, GenreId, Person, PersonId, TitleEnrichment};
use domain::repository::CatalogRepository;

use crate::page::paginate;

#[derive(Default, Clone)]
struct VersionTracks {
    video: Vec<VideoTrack>,
    audio: Vec<AudioTrack>,
    subtitles: Vec<EmbeddedSubtitleTrack>,
    chapters: Vec<Chapter>,
}

#[derive(Default)]
struct State {
    movies: Vec<Movie>,
    series: Vec<Series>,
    seasons: Vec<Season>,
    episodes: Vec<Episode>,
    collections: Vec<Collection>,
    versions: Vec<Version>,
    people: Vec<Person>,
    enrichment: HashMap<TitleRef, TitleEnrichment>,
    artwork: HashMap<ArtworkOwner, Vec<ArtworkRef>>,
    tracks: HashMap<VersionId, VersionTracks>,
    trickplay: HashMap<VersionId, Vec<TrickplayAsset>>,
    markers: HashMap<VersionId, DetectedMarkers>,
}

fn kind_rank(title: &TitleRef) -> u8 {
    match title {
        TitleRef::Movie(_) => 0,
        TitleRef::Series(_) => 1,
    }
}

impl State {
    fn artwork_for(&self, owner: &ArtworkOwner) -> Vec<ArtworkRef> {
        self.artwork.get(owner).cloned().unwrap_or_default()
    }

    fn hydrate_movie(&self, mut movie: Movie) -> Movie {
        movie.artwork = self.artwork_for(&ArtworkOwner::Movie(movie.id.clone()));
        movie
    }

    fn hydrate_series(&self, mut series: Series) -> Series {
        series.artwork = self.artwork_for(&ArtworkOwner::Series(series.id.clone()));
        series
    }

    fn hydrate_season(&self, mut season: Season) -> Season {
        season.artwork = self.artwork_for(&ArtworkOwner::Season(season.id.clone()));
        season
    }

    fn hydrate_episode(&self, mut episode: Episode) -> Episode {
        episode.artwork = self.artwork_for(&ArtworkOwner::Episode(episode.id.clone()));
        episode
    }

    fn hydrate_collection(&self, mut collection: Collection) -> Collection {
        collection.artwork = self.artwork_for(&ArtworkOwner::Collection(collection.id.clone()));
        collection
    }

    fn credited(&self, credits: &[Credit]) -> Vec<CreditedPerson> {
        credits
            .iter()
            .filter_map(|credit| {
                let person = self.people.iter().find(|p| p.id == credit.person)?.clone();
                Some(CreditedPerson {
                    person,
                    role: credit.role,
                    character: credit.character.clone(),
                    order: credit.order,
                })
            })
            .collect()
    }

    fn enrichment_for(&self, owner: &TitleRef) -> TitleEnrichment {
        self.enrichment.get(owner).cloned().unwrap_or_default()
    }
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

    pub fn seed_markers(&self, version: &VersionId, markers: DetectedMarkers) {
        self.state
            .lock()
            .unwrap()
            .markers
            .insert(version.clone(), markers);
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
        let state = self.state.lock().unwrap();
        let movies: Vec<Movie> = state
            .movies
            .iter()
            .cloned()
            .map(|m| state.hydrate_movie(m))
            .collect();
        Ok(paginate(&movies, page))
    }

    async fn get_movie(&self, id: &MovieId) -> Result<Option<Movie>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        Ok(state
            .movies
            .iter()
            .find(|m| &m.id == id)
            .cloned()
            .map(|m| state.hydrate_movie(m)))
    }

    async fn list_series(&self, page: PageRequest) -> Result<Page<Series>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        let series: Vec<Series> = state
            .series
            .iter()
            .cloned()
            .map(|s| state.hydrate_series(s))
            .collect();
        Ok(paginate(&series, page))
    }

    async fn get_series(&self, id: &SeriesId) -> Result<Option<Series>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        Ok(state
            .series
            .iter()
            .find(|s| &s.id == id)
            .cloned()
            .map(|s| state.hydrate_series(s)))
    }

    async fn list_seasons(&self, series: &SeriesId) -> Result<Vec<Season>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        Ok(state
            .seasons
            .iter()
            .filter(|s| &s.series == series)
            .cloned()
            .map(|s| state.hydrate_season(s))
            .collect())
    }

    async fn list_episodes(&self, season: &SeasonId) -> Result<Vec<Episode>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        Ok(state
            .episodes
            .iter()
            .filter(|e| &e.season == season)
            .cloned()
            .map(|e| state.hydrate_episode(e))
            .collect())
    }

    async fn get_episode(&self, id: &EpisodeId) -> Result<Option<Episode>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        Ok(state
            .episodes
            .iter()
            .find(|e| &e.id == id)
            .cloned()
            .map(|e| state.hydrate_episode(e)))
    }

    async fn list_collections(
        &self,
        page: PageRequest,
    ) -> Result<Page<Collection>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        let collections: Vec<Collection> = state
            .collections
            .iter()
            .cloned()
            .map(|c| state.hydrate_collection(c))
            .collect();
        Ok(paginate(&collections, page))
    }

    async fn get_collection(
        &self,
        id: &CollectionId,
    ) -> Result<Option<Collection>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        Ok(state
            .collections
            .iter()
            .find(|c| &c.id == id)
            .cloned()
            .map(|c| state.hydrate_collection(c)))
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
        let state = self.state.lock().unwrap();
        Ok(state
            .seasons
            .iter()
            .find(|s| &s.id == id)
            .cloned()
            .map(|s| state.hydrate_season(s)))
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
        let state = self.state.lock().unwrap();
        let Some(version) = state.versions.iter().find(|v| &v.id == id).cloned() else {
            return Ok(None);
        };
        let tracks = state.tracks.get(id).cloned().unwrap_or_default();
        Ok(Some(VersionDetail {
            version,
            video: tracks.video,
            audio: tracks.audio,
            subtitles: tracks.subtitles,
            chapters: tracks.chapters,
            markers: state.markers.get(id).cloned().unwrap_or_default(),
            trickplay: state.trickplay.get(id).cloned().unwrap_or_default(),
        }))
    }

    async fn upsert_movie(&self, movie: Movie) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if let Some(existing) = state.movies.iter_mut().find(|m| m.id == movie.id) {
            *existing = movie;
        } else {
            state.movies.push(movie);
        }
        Ok(())
    }

    async fn upsert_series(&self, series: Series) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if let Some(existing) = state.series.iter_mut().find(|s| s.id == series.id) {
            *existing = series;
        } else {
            state.series.push(series);
        }
        Ok(())
    }

    async fn upsert_season(&self, season: Season) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if let Some(existing) = state.seasons.iter_mut().find(|s| s.id == season.id) {
            *existing = season;
        } else {
            state.seasons.push(season);
        }
        Ok(())
    }

    async fn upsert_episode(&self, episode: Episode) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if let Some(existing) = state.episodes.iter_mut().find(|e| e.id == episode.id) {
            *existing = episode;
        } else {
            state.episodes.push(episode);
        }
        Ok(())
    }

    async fn upsert_version(&self, version: Version) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if let Some(existing) = state.versions.iter_mut().find(|v| v.id == version.id) {
            *existing = version;
        } else {
            state.versions.push(version);
        }
        Ok(())
    }

    async fn reconcile_library_versions(
        &self,
        library: &LibraryId,
        present_paths: &[String],
    ) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        for version in state.versions.iter_mut() {
            if version.library == *library {
                version.available = present_paths.contains(&version.path);
            }
        }
        Ok(())
    }

    async fn set_artwork(
        &self,
        owner: &ArtworkOwner,
        refs: &[ArtworkRef],
    ) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state
            .lock()
            .unwrap()
            .artwork
            .insert(owner.clone(), refs.to_vec());
        Ok(())
    }

    async fn list_artwork(&self, owner: &ArtworkOwner) -> Result<Vec<ArtworkRef>, RepositoryError> {
        self.guard()?;
        Ok(self.state.lock().unwrap().artwork_for(owner))
    }

    async fn set_version_tracks(
        &self,
        version: &VersionId,
        video: &[VideoTrack],
        audio: &[AudioTrack],
        subtitles: &[EmbeddedSubtitleTrack],
        chapters: &[Chapter],
    ) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state.lock().unwrap().tracks.insert(
            version.clone(),
            VersionTracks {
                video: video.to_vec(),
                audio: audio.to_vec(),
                subtitles: subtitles.to_vec(),
                chapters: chapters.to_vec(),
            },
        );
        Ok(())
    }

    async fn set_trickplay(
        &self,
        version: &VersionId,
        assets: &[TrickplayAsset],
    ) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state
            .lock()
            .unwrap()
            .trickplay
            .insert(version.clone(), assets.to_vec());
        Ok(())
    }

    async fn upsert_person(&self, person: Person) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if let Some(existing) = state.people.iter_mut().find(|p| p.id == person.id) {
            *existing = person;
        } else {
            state.people.push(person);
        }
        Ok(())
    }

    async fn get_person(&self, id: &PersonId) -> Result<Option<Person>, RepositoryError> {
        self.guard()?;
        Ok(self
            .state
            .lock()
            .unwrap()
            .people
            .iter()
            .find(|p| &p.id == id)
            .cloned())
    }

    async fn set_title_enrichment(
        &self,
        owner: &TitleRef,
        enrichment: &TitleEnrichment,
    ) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state
            .lock()
            .unwrap()
            .enrichment
            .insert(owner.clone(), enrichment.clone());
        Ok(())
    }

    async fn movie_detail(&self, id: &MovieId) -> Result<Option<MovieDetail>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        let Some(movie) = state
            .movies
            .iter()
            .find(|m| &m.id == id)
            .cloned()
            .map(|m| state.hydrate_movie(m))
        else {
            return Ok(None);
        };
        let enrichment = state.enrichment_for(&TitleRef::Movie(id.clone()));
        Ok(Some(MovieDetail {
            movie,
            genres: enrichment.genres,
            credits: state.credited(&enrichment.credits),
            studios: enrichment.studios,
            ratings: enrichment.ratings,
            external_ids: enrichment.external_ids,
            extras: enrichment.extras,
        }))
    }

    async fn series_detail(&self, id: &SeriesId) -> Result<Option<SeriesDetail>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        let Some(series) = state
            .series
            .iter()
            .find(|s| &s.id == id)
            .cloned()
            .map(|s| state.hydrate_series(s))
        else {
            return Ok(None);
        };
        let enrichment = state.enrichment_for(&TitleRef::Series(id.clone()));
        Ok(Some(SeriesDetail {
            series,
            genres: enrichment.genres,
            credits: state.credited(&enrichment.credits),
            studios: enrichment.studios,
            ratings: enrichment.ratings,
            external_ids: enrichment.external_ids,
            extras: enrichment.extras,
        }))
    }

    async fn filmography(&self, id: &PersonId) -> Result<Vec<Credit>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        let mut out: Vec<Credit> = Vec::new();
        for (owner, enrichment) in &state.enrichment {
            for credit in &enrichment.credits {
                if &credit.person == id {
                    out.push(Credit {
                        person: credit.person.clone(),
                        title: owner.clone(),
                        role: credit.role,
                        character: credit.character.clone(),
                        order: credit.order,
                    });
                }
            }
        }
        out.sort_by(|a, b| {
            kind_rank(&a.title)
                .cmp(&kind_rank(&b.title))
                .then_with(|| a.title.id().cmp(b.title.id()))
                .then_with(|| a.order.cmp(&b.order))
        });
        Ok(out)
    }

    async fn list_genres(&self) -> Result<Vec<Genre>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        let mut genres: Vec<Genre> = Vec::new();
        for enrichment in state.enrichment.values() {
            for genre in &enrichment.genres {
                if !genres.iter().any(|existing| existing.id == genre.id) {
                    genres.push(genre.clone());
                }
            }
        }
        genres.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.0.cmp(&b.id.0)));
        Ok(genres)
    }

    async fn list_movies_by_genre(
        &self,
        genre: &GenreId,
        page: PageRequest,
    ) -> Result<Page<Movie>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        let mut matched: Vec<Movie> = state
            .movies
            .iter()
            .filter(|movie| {
                state
                    .enrichment
                    .get(&TitleRef::Movie(movie.id.clone()))
                    .is_some_and(|e| e.genres.iter().any(|g| &g.id == genre))
            })
            .cloned()
            .map(|m| state.hydrate_movie(m))
            .collect();
        matched.sort_by(|a, b| {
            a.added_at
                .cmp(&b.added_at)
                .then_with(|| a.id.0.cmp(&b.id.0))
        });
        Ok(paginate(&matched, page))
    }

    async fn list_series_by_genre(
        &self,
        genre: &GenreId,
        page: PageRequest,
    ) -> Result<Page<Series>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        let mut matched: Vec<Series> = state
            .series
            .iter()
            .filter(|series| {
                state
                    .enrichment
                    .get(&TitleRef::Series(series.id.clone()))
                    .is_some_and(|e| e.genres.iter().any(|g| &g.id == genre))
            })
            .cloned()
            .map(|s| state.hydrate_series(s))
            .collect();
        matched.sort_by(|a, b| {
            a.added_at
                .cmp(&b.added_at)
                .then_with(|| a.id.0.cmp(&b.id.0))
        });
        Ok(paginate(&matched, page))
    }

    async fn titles_in_library(
        &self,
        kind: TitleKind,
        library: &LibraryId,
    ) -> Result<Vec<String>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        let mut ids: Vec<String> = Vec::new();
        for version in state.versions.iter().filter(|v| &v.library == library) {
            let series_id = match (kind, &version.title) {
                (TitleKind::Movie, TitleId::Movie(id)) => Some(id.0.clone()),
                (TitleKind::Series, TitleId::Episode(episode)) => state
                    .episodes
                    .iter()
                    .find(|e| e.id == *episode)
                    .and_then(|e| state.seasons.iter().find(|s| s.id == e.season))
                    .map(|s| s.series.0.clone()),
                _ => None,
            };
            if let Some(id) = series_id
                && !ids.contains(&id)
            {
                ids.push(id);
            }
        }
        ids.sort();
        Ok(ids)
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
            artwork: Vec::new(),
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
            artwork: Vec::new(),
        }
    }

    fn season(id: &str, series: &str) -> Season {
        Season {
            id: SeasonId(id.to_owned()),
            series: SeriesId(series.to_owned()),
            number: 1,
            title: None,
            overview: None,
            artwork: Vec::new(),
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
            artwork: Vec::new(),
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
            available: true,
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
            artwork: Vec::new(),
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
    async fn set_list_and_hydrate_artwork() {
        use domain::catalog::ArtworkId;
        use domain::metadata::ArtworkKind;

        let repo = MockCatalogRepo::new();
        repo.add_movie(movie("m1"));
        repo.add_movie(movie("m2"));

        let poster = ArtworkRef {
            id: ArtworkId("art-1".into()),
            kind: ArtworkKind::Poster,
            widths: vec![180, 480],
        };
        let backdrop = ArtworkRef {
            id: ArtworkId("art-2".into()),
            kind: ArtworkKind::Backdrop,
            widths: vec![960],
        };
        repo.set_artwork(
            &ArtworkOwner::Movie(MovieId("m1".into())),
            &[poster.clone(), backdrop.clone()],
        )
        .await
        .unwrap();

        let listed = repo
            .list_artwork(&ArtworkOwner::Movie(MovieId("m1".into())))
            .await
            .unwrap();
        assert_eq!(listed, vec![poster.clone(), backdrop.clone()]);

        let hydrated = repo
            .get_movie(&MovieId("m1".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(hydrated.artwork, vec![poster.clone(), backdrop.clone()]);

        let listed_movies = repo.list_movies(page()).await.unwrap();
        assert_eq!(
            listed_movies.items[0].artwork,
            vec![poster.clone(), backdrop.clone()]
        );
        assert!(listed_movies.items[1].artwork.is_empty());

        repo.set_artwork(
            &ArtworkOwner::Movie(MovieId("m1".into())),
            std::slice::from_ref(&poster),
        )
        .await
        .unwrap();
        let replaced = repo
            .get_movie(&MovieId("m1".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(replaced.artwork, vec![poster]);

        assert!(
            repo.list_artwork(&ArtworkOwner::Movie(MovieId("m2".into())))
                .await
                .unwrap()
                .is_empty()
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
            artwork: Vec::new(),
        })
        .await
        .unwrap();
        repo.upsert_collection(Collection {
            id: CollectionId("c1".into()),
            name: "Renamed".into(),
            overview: None,
            movies: Vec::new(),
            artwork: Vec::new(),
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
                artwork: Vec::new(),
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
        assert!(
            repo.set_artwork(&ArtworkOwner::Movie(MovieId("m1".into())), &[])
                .await
                .is_err()
        );
        assert!(
            repo.list_artwork(&ArtworkOwner::Movie(MovieId("m1".into())))
                .await
                .is_err()
        );
        assert!(
            repo.set_version_tracks(&VersionId("v1".into()), &[], &[], &[], &[])
                .await
                .is_err()
        );
        assert!(
            repo.set_trickplay(&VersionId("v1".into()), &[])
                .await
                .is_err()
        );
        assert!(repo.upsert_movie(movie("m1")).await.is_err());
        assert!(repo.upsert_series(series("s1")).await.is_err());
        assert!(repo.upsert_season(season("se1", "s1")).await.is_err());
        assert!(repo.upsert_episode(episode("e1", "se1")).await.is_err());
        assert!(repo.upsert_version(version("v1")).await.is_err());
        assert!(
            repo.upsert_person(Person {
                id: PersonId("p1".into()),
                name: "x".into(),
            })
            .await
            .is_err()
        );
        assert!(repo.get_person(&PersonId("p1".into())).await.is_err());
        assert!(
            repo.set_title_enrichment(
                &TitleRef::Movie(MovieId("m1".into())),
                &TitleEnrichment::default()
            )
            .await
            .is_err()
        );
        assert!(repo.movie_detail(&MovieId("m1".into())).await.is_err());
        assert!(repo.series_detail(&SeriesId("s1".into())).await.is_err());
        assert!(repo.filmography(&PersonId("p1".into())).await.is_err());
        assert!(repo.list_genres().await.is_err());
        assert!(
            repo.list_movies_by_genre(&GenreId("g1".into()), page())
                .await
                .is_err()
        );
        assert!(
            repo.list_series_by_genre(&GenreId("g1".into()), page())
                .await
                .is_err()
        );
        assert!(
            repo.titles_in_library(TitleKind::Movie, &LibraryId("lib1".into()))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn titles_in_library_is_kind_aware() {
        let repo = MockCatalogRepo::new();
        repo.add_season(season("se1", "s1"));
        repo.add_episode(episode("e1", "se1"));
        repo.add_version(Version {
            id: VersionId("mv1".into()),
            title: TitleId::Movie(MovieId("m1".into())),
            library: LibraryId("lib1".into()),
            ..version("mv1")
        });
        repo.add_version(Version {
            id: VersionId("ev1".into()),
            title: TitleId::Episode(EpisodeId("e1".into())),
            library: LibraryId("lib2".into()),
            ..version("ev1")
        });

        assert_eq!(
            repo.titles_in_library(TitleKind::Movie, &LibraryId("lib1".into()))
                .await
                .unwrap(),
            vec!["m1".to_owned()]
        );
        assert!(
            repo.titles_in_library(TitleKind::Movie, &LibraryId("lib2".into()))
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            repo.titles_in_library(TitleKind::Series, &LibraryId("lib2".into()))
                .await
                .unwrap(),
            vec!["s1".to_owned()]
        );
        assert!(
            repo.titles_in_library(TitleKind::Series, &LibraryId("lib1".into()))
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn upsert_replaces_by_id_and_is_idempotent() {
        let repo = MockCatalogRepo::new();
        repo.upsert_movie(movie("m1")).await.unwrap();
        repo.upsert_movie(Movie {
            title: "renamed".into(),
            ..movie("m1")
        })
        .await
        .unwrap();
        assert_eq!(repo.list_movies(page()).await.unwrap().total, 1);
        assert_eq!(
            repo.get_movie(&MovieId("m1".into()))
                .await
                .unwrap()
                .unwrap()
                .title,
            "renamed"
        );

        repo.upsert_series(series("s1")).await.unwrap();
        repo.upsert_series(series("s1")).await.unwrap();
        assert_eq!(repo.list_series(page()).await.unwrap().total, 1);

        repo.upsert_season(season("se1", "s1")).await.unwrap();
        repo.upsert_season(season("se1", "s1")).await.unwrap();
        assert_eq!(
            repo.list_seasons(&SeriesId("s1".into()))
                .await
                .unwrap()
                .len(),
            1
        );

        repo.upsert_episode(episode("e1", "se1")).await.unwrap();
        repo.upsert_episode(episode("e1", "se1")).await.unwrap();
        assert_eq!(
            repo.list_episodes(&SeasonId("se1".into()))
                .await
                .unwrap()
                .len(),
            1
        );

        repo.upsert_version(version("v1")).await.unwrap();
        repo.upsert_version(version("v1")).await.unwrap();
        assert_eq!(
            repo.list_versions(&TitleId::Movie(MovieId("m1".into())), page())
                .await
                .unwrap()
                .total,
            1
        );
    }

    #[tokio::test]
    async fn enrichment_details_people_genres_and_filmography() {
        use domain::catalog::TitleRef;
        use domain::metadata::{
            Credit, CreditRole, ExternalId, Extra, ExtraKind, Genre, GenreId, Person, PersonId,
            Rating, Studio, StudioId, TitleEnrichment,
        };

        let repo = MockCatalogRepo::new();
        repo.add_movie(movie("m1"));
        repo.add_series(series("s1"));
        repo.upsert_person(Person {
            id: PersonId("p1".into()),
            name: "Ada".into(),
        })
        .await
        .unwrap();
        repo.upsert_person(Person {
            id: PersonId("p2".into()),
            name: "Bob".into(),
        })
        .await
        .unwrap();

        let movie_enrichment = TitleEnrichment {
            genres: vec![
                Genre {
                    id: GenreId("g2".into()),
                    name: "Drama".into(),
                },
                Genre {
                    id: GenreId("g1".into()),
                    name: "Action".into(),
                },
            ],
            credits: vec![
                Credit {
                    person: PersonId("p1".into()),
                    title: TitleRef::Movie(MovieId("m1".into())),
                    role: CreditRole::Actor,
                    character: Some("Hero".into()),
                    order: 0,
                },
                Credit {
                    person: PersonId("p2".into()),
                    title: TitleRef::Movie(MovieId("m1".into())),
                    role: CreditRole::Director,
                    character: None,
                    order: 1,
                },
                Credit {
                    person: PersonId("ghost".into()),
                    title: TitleRef::Movie(MovieId("m1".into())),
                    role: CreditRole::Writer,
                    character: None,
                    order: 2,
                },
            ],
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
                value: "100".into(),
            }],
            extras: vec![Extra {
                kind: ExtraKind::Trailer,
                title: "Teaser".into(),
                path: "/t.mkv".into(),
            }],
        };
        repo.set_title_enrichment(&TitleRef::Movie(MovieId("m1".into())), &movie_enrichment)
            .await
            .unwrap();
        repo.set_title_enrichment(
            &TitleRef::Series(SeriesId("s1".into())),
            &TitleEnrichment {
                genres: vec![Genre {
                    id: GenreId("g1".into()),
                    name: "Action".into(),
                }],
                credits: vec![Credit {
                    person: PersonId("p1".into()),
                    title: TitleRef::Series(SeriesId("s1".into())),
                    role: CreditRole::Actor,
                    character: Some("Guest".into()),
                    order: 0,
                }],
                ..TitleEnrichment::default()
            },
        )
        .await
        .unwrap();

        let detail = repo
            .movie_detail(&MovieId("m1".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(detail.genres.len(), 2);
        assert_eq!(detail.credits.len(), 2);
        assert_eq!(detail.credits[0].person.name, "Ada");
        assert_eq!(detail.credits[0].role, CreditRole::Actor);
        assert_eq!(detail.credits[0].character, Some("Hero".to_owned()));
        assert_eq!(detail.credits[1].person.name, "Bob");
        assert_eq!(detail.studios[0].name, "Acme");
        assert_eq!(detail.ratings[0].value, 8.5);
        assert_eq!(detail.external_ids[0].value, "100");
        assert_eq!(detail.extras[0].title, "Teaser");
        assert!(
            repo.movie_detail(&MovieId("nope".into()))
                .await
                .unwrap()
                .is_none()
        );

        let series_detail = repo
            .series_detail(&SeriesId("s1".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(series_detail.genres.len(), 1);
        assert_eq!(series_detail.credits.len(), 1);
        assert_eq!(series_detail.credits[0].character, Some("Guest".to_owned()));
        assert!(
            repo.series_detail(&SeriesId("nope".into()))
                .await
                .unwrap()
                .is_none()
        );

        let p1_film = repo.filmography(&PersonId("p1".into())).await.unwrap();
        assert_eq!(p1_film.len(), 2);
        assert_eq!(p1_film[0].title, TitleRef::Movie(MovieId("m1".into())));
        assert_eq!(p1_film[1].title, TitleRef::Series(SeriesId("s1".into())));
        let p2_film = repo.filmography(&PersonId("p2".into())).await.unwrap();
        assert_eq!(p2_film.len(), 1);
        assert_eq!(p2_film[0].role, CreditRole::Director);

        assert_eq!(
            repo.get_person(&PersonId("p1".into()))
                .await
                .unwrap()
                .unwrap()
                .name,
            "Ada"
        );
        assert!(
            repo.get_person(&PersonId("nope".into()))
                .await
                .unwrap()
                .is_none()
        );

        let genres = repo.list_genres().await.unwrap();
        assert_eq!(
            genres.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(),
            ["Action", "Drama"]
        );

        let by_g1 = repo
            .list_movies_by_genre(&GenreId("g1".into()), page())
            .await
            .unwrap();
        assert_eq!(
            by_g1
                .items
                .iter()
                .map(|m| m.id.0.as_str())
                .collect::<Vec<_>>(),
            ["m1"]
        );
        assert_eq!(
            repo.list_movies_by_genre(&GenreId("g2".into()), page())
                .await
                .unwrap()
                .total,
            1
        );
        assert_eq!(
            repo.list_movies_by_genre(&GenreId("nope".into()), page())
                .await
                .unwrap()
                .total,
            0
        );
        assert_eq!(
            repo.list_series_by_genre(&GenreId("g1".into()), page())
                .await
                .unwrap()
                .items
                .iter()
                .map(|s| s.id.0.as_str())
                .collect::<Vec<_>>(),
            ["s1"]
        );
        assert_eq!(
            repo.list_series_by_genre(&GenreId("g2".into()), page())
                .await
                .unwrap()
                .total,
            0
        );

        repo.set_title_enrichment(
            &TitleRef::Movie(MovieId("m1".into())),
            &TitleEnrichment::default(),
        )
        .await
        .unwrap();
        let cleared = repo
            .movie_detail(&MovieId("m1".into()))
            .await
            .unwrap()
            .unwrap();
        assert!(cleared.genres.is_empty());
        assert!(cleared.credits.is_empty());
    }

    #[tokio::test]
    async fn set_version_tracks_and_trickplay_hydrate_version_detail() {
        use domain::media::{IntroMarker, SubtitleFormat};

        let repo = MockCatalogRepo::new();
        repo.add_version(version("v1"));
        let v1 = VersionId("v1".to_owned());

        repo.set_version_tracks(
            &v1,
            &[VideoTrack {
                index: 0,
                codec: "h264".into(),
                width: 1920,
                height: 1080,
                bit_depth: 8,
                hdr: None,
                frame_rate: 24.0,
                bitrate: None,
            }],
            &[AudioTrack {
                index: 1,
                codec: "aac".into(),
                channels: 2,
                language: None,
                bitrate: None,
            }],
            &[EmbeddedSubtitleTrack {
                index: 2,
                language: None,
                format: SubtitleFormat::Srt,
                forced: false,
                default: true,
            }],
            &[Chapter {
                title: "One".into(),
                start_ms: 0,
            }],
        )
        .await
        .unwrap();
        repo.set_trickplay(
            &v1,
            &[TrickplayAsset {
                version: v1.clone(),
                interval_ms: 10_000,
                columns: 10,
                rows: 10,
                tile_width: 320,
                tile_height: 180,
                sheet_paths: vec!["/tp/v1/sheet-001.jpg".into()],
            }],
        )
        .await
        .unwrap();
        repo.seed_markers(
            &v1,
            DetectedMarkers {
                intros: vec![IntroMarker {
                    version: v1.clone(),
                    start_ms: 0,
                    end_ms: 5_000,
                }],
                credits: Vec::new(),
            },
        );

        let detail = repo.version_detail(&v1).await.unwrap().unwrap();
        assert_eq!(detail.video.len(), 1);
        assert_eq!(detail.audio.len(), 1);
        assert_eq!(detail.subtitles.len(), 1);
        assert_eq!(detail.chapters.len(), 1);
        assert_eq!(detail.trickplay.len(), 1);
        assert_eq!(detail.trickplay[0].columns, 10);
        assert_eq!(detail.markers.intros.len(), 1);

        repo.set_version_tracks(&v1, &[], &[], &[], &[])
            .await
            .unwrap();
        repo.set_trickplay(&v1, &[]).await.unwrap();
        let cleared = repo.version_detail(&v1).await.unwrap().unwrap();
        assert!(cleared.video.is_empty());
        assert!(cleared.trickplay.is_empty());

        assert!(
            repo.version_detail(&VersionId("nope".into()))
                .await
                .unwrap()
                .is_none()
        );
    }
}

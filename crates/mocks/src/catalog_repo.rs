use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use domain::catalog::{
    ArtworkId, ArtworkOwner, ArtworkRef, Collection, CollectionId, Episode, EpisodeContext,
    EpisodeId, Movie, MovieDetail, MovieId, RandomScope, Season, SeasonId, Series, SeriesDetail,
    SeriesId, TitleId, TitleKind, TitleListFilter, TitleRef, Version, VersionDetail, VersionId,
};
use domain::common::{Page, PageRequest};
use domain::error::RepositoryError;
use domain::library::LibraryId;
use domain::media::{
    AudioTrack, Chapter, DetectedMarkers, EmbeddedSubtitleTrack, SubtitleFile, TrickplayAsset,
    VideoTrack,
};
use domain::metadata::{
    ContentRating, Credit, CreditedPerson, Genre, Person, PersonId, TitleEnrichment,
};
use domain::repository::CatalogRepository;

use domain::common::paginate;

use crate::sort::sort_titles;

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
    subtitle_files: HashMap<VersionId, Vec<SubtitleFile>>,
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
    fail_writes: Arc<AtomicBool>,
    episode_lookups: Arc<AtomicUsize>,
    detail_lookups: Arc<AtomicUsize>,
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
        let mut state = self.state.lock().unwrap();
        remember_artwork(&mut state, ArtworkOwner::Season(season.id.clone()), &season.artwork);
        state.seasons.push(season);
    }

    pub fn add_episode(&self, episode: Episode) {
        let mut state = self.state.lock().unwrap();
        remember_artwork(&mut state, ArtworkOwner::Episode(episode.id.clone()), &episode.artwork);
        state.episodes.push(episode);
    }

    pub fn add_collection(&self, collection: Collection) {
        let mut state = self.state.lock().unwrap();
        remember_artwork(
            &mut state,
            ArtworkOwner::Collection(collection.id.clone()),
            &collection.artwork,
        );
        state.collections.push(collection);
    }

    pub fn add_version(&self, version: Version) {
        self.state.lock().unwrap().versions.push(version);
    }

    pub fn add_person(&self, person: Person) {
        self.state.lock().unwrap().people.push(person);
    }

    pub fn seed_movie_detail(&self, detail: MovieDetail) {
        let id = detail.movie.id.clone();
        let owner = TitleRef::Movie(id.clone());
        let mut state = self.state.lock().unwrap();
        remember_artwork(&mut state, ArtworkOwner::Movie(id.clone()), &detail.movie.artwork);
        if !state.movies.iter().any(|m| m.id == id) {
            state.movies.push(detail.movie);
        }
        let credits = split_credits(&mut state, &owner, detail.credits);
        state.enrichment.insert(
            owner,
            TitleEnrichment {
                genres: detail.genres,
                credits,
                studios: detail.studios,
                ratings: detail.ratings,
                external_ids: detail.external_ids,
                extras: detail.extras,
            },
        );
    }

    pub fn seed_series_detail(&self, detail: SeriesDetail) {
        let id = detail.series.id.clone();
        let owner = TitleRef::Series(id.clone());
        let mut state = self.state.lock().unwrap();
        remember_artwork(&mut state, ArtworkOwner::Series(id.clone()), &detail.series.artwork);
        if !state.series.iter().any(|s| s.id == id) {
            state.series.push(detail.series);
        }
        let credits = split_credits(&mut state, &owner, detail.credits);
        state.enrichment.insert(
            owner,
            TitleEnrichment {
                genres: detail.genres,
                credits,
                studios: detail.studios,
                ratings: detail.ratings,
                external_ids: detail.external_ids,
                extras: detail.extras,
            },
        );
    }

    pub fn seed_version_detail(&self, detail: VersionDetail) {
        let id = detail.version.id.clone();
        let mut state = self.state.lock().unwrap();
        if !state.versions.iter().any(|v| v.id == id) {
            state.versions.push(detail.version);
        }
        state.tracks.insert(
            id.clone(),
            VersionTracks {
                video: detail.video,
                audio: detail.audio,
                subtitles: detail.subtitles,
                chapters: detail.chapters,
            },
        );
        state.subtitle_files.insert(id.clone(), detail.subtitle_files);
        state.trickplay.insert(id.clone(), detail.trickplay);
        state.markers.insert(id, detail.markers);
    }

    pub fn seed_markers(&self, version: &VersionId, markers: DetectedMarkers) {
        self.state.lock().unwrap().markers.insert(version.clone(), markers);
    }

    pub fn set_fail(&self) {
        self.fail.store(true, Ordering::Relaxed);
    }

    pub fn set_fail_writes(&self) {
        self.fail_writes.store(true, Ordering::Relaxed);
    }

    fn guard(&self) -> Result<(), RepositoryError> {
        if self.fail.load(Ordering::Relaxed) {
            Err(RepositoryError::Backend("mock catalog failure".to_owned()))
        } else {
            Ok(())
        }
    }

    fn counted(&self) -> Result<(), RepositoryError> {
        self.episode_lookups.fetch_add(1, Ordering::Relaxed);
        self.guard()
    }

    pub fn episode_lookup_count(&self) -> usize {
        self.episode_lookups.load(Ordering::Relaxed)
    }

    pub fn detail_lookup_count(&self) -> usize {
        self.detail_lookups.load(Ordering::Relaxed)
    }
}

impl CatalogRepository for MockCatalogRepo {
    async fn list_movies(&self, page: PageRequest) -> Result<Page<Movie>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        let movies: Vec<Movie> =
            state.movies.iter().cloned().map(|m| state.hydrate_movie(m)).collect();
        Ok(paginate(&movies, page))
    }

    async fn get_movie(&self, id: &MovieId) -> Result<Option<Movie>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        Ok(state.movies.iter().find(|m| &m.id == id).cloned().map(|m| state.hydrate_movie(m)))
    }

    async fn movies_by_ids(&self, ids: &[MovieId]) -> Result<Vec<Movie>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        Ok(ids
            .iter()
            .filter_map(|id| state.movies.iter().find(|m| &m.id == id).cloned())
            .map(|m| state.hydrate_movie(m))
            .collect())
    }

    async fn series_by_ids(&self, ids: &[SeriesId]) -> Result<Vec<Series>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        Ok(ids
            .iter()
            .filter_map(|id| state.series.iter().find(|s| &s.id == id).cloned())
            .map(|s| state.hydrate_series(s))
            .collect())
    }

    async fn series_versions(&self, series: &SeriesId) -> Result<Vec<Version>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        let mut seasons: Vec<_> =
            state.seasons.iter().filter(|season| &season.series == series).collect();
        seasons.sort_by_key(|season| season.number);
        let mut found = Vec::new();
        for season in seasons {
            let mut episodes: Vec<_> =
                state.episodes.iter().filter(|episode| episode.season == season.id).collect();
            episodes.sort_by(|a, b| a.number.cmp(&b.number).then_with(|| a.id.0.cmp(&b.id.0)));
            for episode in episodes {
                found.extend(
                    state
                        .versions
                        .iter()
                        .filter(|version| version.title == TitleId::Episode(episode.id.clone()))
                        .cloned(),
                );
            }
        }
        Ok(found)
    }

    async fn list_series(&self, page: PageRequest) -> Result<Page<Series>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        let series: Vec<Series> =
            state.series.iter().cloned().map(|s| state.hydrate_series(s)).collect();
        Ok(paginate(&series, page))
    }

    async fn get_series(&self, id: &SeriesId) -> Result<Option<Series>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        Ok(state.series.iter().find(|s| &s.id == id).cloned().map(|s| state.hydrate_series(s)))
    }

    async fn list_seasons(&self, series: &SeriesId) -> Result<Vec<Season>, RepositoryError> {
        self.counted()?;
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
        self.counted()?;
        let state = self.state.lock().unwrap();
        Ok(state
            .episodes
            .iter()
            .filter(|e| &e.season == season)
            .cloned()
            .map(|e| state.hydrate_episode(e))
            .collect())
    }

    async fn episode_ids_for_series(
        &self,
        series: &[SeriesId],
    ) -> Result<HashMap<SeriesId, Vec<EpisodeId>>, RepositoryError> {
        self.counted()?;
        let state = self.state.lock().unwrap();
        let mut out: HashMap<SeriesId, Vec<EpisodeId>> = HashMap::new();
        for season in &state.seasons {
            if !series.contains(&season.series) {
                continue;
            }
            for episode in state.episodes.iter().filter(|e| e.season == season.id) {
                out.entry(season.series.clone()).or_default().push(episode.id.clone());
            }
        }
        Ok(out)
    }

    async fn episode_ids_for_seasons(
        &self,
        seasons: &[SeasonId],
    ) -> Result<HashMap<SeasonId, Vec<EpisodeId>>, RepositoryError> {
        self.counted()?;
        let state = self.state.lock().unwrap();
        let mut out: HashMap<SeasonId, Vec<EpisodeId>> = HashMap::new();
        for episode in &state.episodes {
            if seasons.contains(&episode.season) {
                out.entry(episode.season.clone()).or_default().push(episode.id.clone());
            }
        }
        Ok(out)
    }

    async fn recent_episodes(
        &self,
        filter: &TitleListFilter,
        limit: u32,
    ) -> Result<Vec<EpisodeContext>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        let mut matched = visible_contexts(&state, filter);
        matched.sort_by(|a, b| {
            b.episode
                .added_at
                .cmp(&a.episode.added_at)
                .then_with(|| a.episode.id.0.cmp(&b.episode.id.0))
        });
        matched.truncate(limit as usize);
        Ok(matched)
    }

    async fn visible_movies(
        &self,
        ids: &[MovieId],
        filter: &TitleListFilter,
    ) -> Result<Vec<Movie>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        Ok(state
            .movies
            .iter()
            .filter(|movie| {
                ids.contains(&movie.id)
                    && !rating_blocked(movie.content_rating.as_ref(), &filter.blocked_ratings)
                    && movie_in_libraries(&state, &movie.id, filter.libraries.as_ref())
            })
            .cloned()
            .map(|m| state.hydrate_movie(m))
            .collect())
    }

    async fn visible_episodes(
        &self,
        ids: &[EpisodeId],
        filter: &TitleListFilter,
    ) -> Result<Vec<EpisodeContext>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        Ok(visible_contexts(&state, filter)
            .into_iter()
            .filter(|entry| ids.contains(&entry.episode.id))
            .collect())
    }

    async fn next_episode_in_series(
        &self,
        series: &SeriesId,
        after_season: u16,
        after_number: u16,
        filter: &TitleListFilter,
    ) -> Result<Option<EpisodeContext>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        let mut later: Vec<EpisodeContext> = visible_contexts(&state, filter)
            .into_iter()
            .filter(|entry| {
                &entry.series == series
                    && (entry.season_number, entry.episode.number) > (after_season, after_number)
            })
            .collect();
        later.sort_by(|a, b| {
            (a.season_number, a.episode.number, &a.episode.id.0).cmp(&(
                b.season_number,
                b.episode.number,
                &b.episode.id.0,
            ))
        });
        Ok(later.into_iter().next())
    }

    async fn get_episode(&self, id: &EpisodeId) -> Result<Option<Episode>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        Ok(state.episodes.iter().find(|e| &e.id == id).cloned().map(|e| state.hydrate_episode(e)))
    }

    async fn list_collections(
        &self,
        page: PageRequest,
    ) -> Result<Page<Collection>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        let mut collections: Vec<Collection> =
            state.collections.iter().cloned().map(|c| state.hydrate_collection(c)).collect();
        collections.sort_by(|a, b| {
            a.name.to_lowercase().cmp(&b.name.to_lowercase()).then_with(|| a.id.0.cmp(&b.id.0))
        });
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
            let mut collection = collection;
            collection.added_at = existing.added_at;
            *existing = collection;
        } else {
            state.collections.push(collection);
        }
        Ok(())
    }

    async fn delete_collection(&self, id: &CollectionId) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state.lock().unwrap().collections.retain(|c| &c.id != id);
        Ok(())
    }

    async fn collections_of_movie(
        &self,
        movie: &MovieId,
    ) -> Result<Vec<CollectionId>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        let mut owners: Vec<CollectionId> = state
            .collections
            .iter()
            .filter(|c| c.movies.contains(movie))
            .map(|c| c.id.clone())
            .collect();
        owners.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(owners)
    }

    async fn get_season(&self, id: &SeasonId) -> Result<Option<Season>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        Ok(state.seasons.iter().find(|s| &s.id == id).cloned().map(|s| state.hydrate_season(s)))
    }

    async fn list_versions(
        &self,
        title: &TitleId,
        page: PageRequest,
    ) -> Result<Page<Version>, RepositoryError> {
        self.guard()?;
        let mut matched: Vec<Version> = self
            .state
            .lock()
            .unwrap()
            .versions
            .iter()
            .filter(|v| &v.title == title)
            .cloned()
            .collect();
        matched.sort_by(|a, b| a.id.0.cmp(&b.id.0));
        Ok(paginate(&matched, page))
    }

    async fn list_library_versions(
        &self,
        library: &LibraryId,
        page: PageRequest,
    ) -> Result<Page<Version>, RepositoryError> {
        self.guard()?;
        let mut matched: Vec<Version> = self
            .state
            .lock()
            .unwrap()
            .versions
            .iter()
            .filter(|v| &v.library == library)
            .cloned()
            .collect();
        matched.sort_by(|a, b| a.id.0.cmp(&b.id.0));
        Ok(paginate(&matched, page))
    }

    async fn list_all_versions(&self, page: PageRequest) -> Result<Page<Version>, RepositoryError> {
        self.guard()?;
        let mut all: Vec<Version> = self.state.lock().unwrap().versions.to_vec();
        all.sort_by(|a, b| a.id.0.cmp(&b.id.0));
        Ok(paginate(&all, page))
    }

    async fn version_detail(
        &self,
        id: &VersionId,
    ) -> Result<Option<VersionDetail>, RepositoryError> {
        self.detail_lookups.fetch_add(1, Ordering::Relaxed);
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
            subtitle_files: state.subtitle_files.get(id).cloned().unwrap_or_default(),
            chapters: tracks.chapters,
            markers: state.markers.get(id).cloned().unwrap_or_default(),
            trickplay: state.trickplay.get(id).cloned().unwrap_or_default(),
        }))
    }

    async fn get_version(&self, id: &VersionId) -> Result<Option<Version>, RepositoryError> {
        self.guard()?;
        let found = self.state.lock().unwrap().versions.iter().find(|v| &v.id == id).cloned();
        Ok(found)
    }

    async fn delete_version(&self, id: &VersionId) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        state.versions.retain(|v| &v.id != id);
        state.tracks.remove(id);
        state.subtitle_files.remove(id);
        state.markers.remove(id);
        state.trickplay.remove(id);
        Ok(())
    }

    async fn delete_movie(&self, id: &MovieId) -> Result<bool, RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        let title = TitleId::Movie(id.clone());
        if state.versions.iter().any(|v| v.title == title) {
            return Ok(false);
        }
        state.movies.retain(|m| &m.id != id);
        state.enrichment.remove(&TitleRef::Movie(id.clone()));
        state.artwork.remove(&ArtworkOwner::Movie(id.clone()));
        for collection in &mut state.collections {
            collection.movies.retain(|m| m != id);
        }
        Ok(true)
    }

    async fn delete_series(&self, id: &SeriesId) -> Result<bool, RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if state.seasons.iter().any(|s| &s.series == id) {
            return Ok(false);
        }
        state.series.retain(|s| &s.id != id);
        state.enrichment.remove(&TitleRef::Series(id.clone()));
        state.artwork.remove(&ArtworkOwner::Series(id.clone()));
        Ok(true)
    }

    async fn delete_season(&self, id: &SeasonId) -> Result<bool, RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if state.episodes.iter().any(|e| &e.season == id) {
            return Ok(false);
        }
        state.seasons.retain(|s| &s.id != id);
        state.artwork.remove(&ArtworkOwner::Season(id.clone()));
        Ok(true)
    }

    async fn delete_episode(&self, id: &EpisodeId) -> Result<bool, RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        let title = TitleId::Episode(id.clone());
        if state.versions.iter().any(|v| v.title == title) {
            return Ok(false);
        }
        state.episodes.retain(|e| &e.id != id);
        state.artwork.remove(&ArtworkOwner::Episode(id.clone()));
        Ok(true)
    }

    async fn upsert_movie(&self, movie: Movie) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if let Some(existing) = state.movies.iter_mut().find(|m| m.id == movie.id) {
            let mut movie = movie;
            movie.added_at = existing.added_at;
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
            let mut series = series;
            series.added_at = existing.added_at;
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
            let mut season = season;
            season.added_at = existing.added_at;
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
            let mut episode = episode;
            episode.added_at = existing.added_at;
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
            let mut version = version;
            version.added_at = existing.added_at;
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
        self.state.lock().unwrap().artwork.insert(owner.clone(), refs.to_vec());
        Ok(())
    }

    async fn list_artwork(&self, owner: &ArtworkOwner) -> Result<Vec<ArtworkRef>, RepositoryError> {
        self.guard()?;
        Ok(self.state.lock().unwrap().artwork_for(owner))
    }

    async fn all_artwork_ids(&self) -> Result<Vec<ArtworkId>, RepositoryError> {
        self.guard()?;
        Ok(self
            .state
            .lock()
            .unwrap()
            .artwork
            .values()
            .flatten()
            .map(|art| art.id.clone())
            .collect())
    }

    async fn all_version_ids(&self) -> Result<Vec<VersionId>, RepositoryError> {
        self.guard()?;
        Ok(self.state.lock().unwrap().versions.iter().map(|version| version.id.clone()).collect())
    }

    async fn live_artwork_ids(&self, ids: &[String]) -> Result<HashSet<String>, RepositoryError> {
        let live = self.all_artwork_ids().await?;
        Ok(ids.iter().filter(|id| live.iter().any(|art| &art.0 == *id)).cloned().collect())
    }

    async fn live_version_ids(&self, ids: &[String]) -> Result<HashSet<String>, RepositoryError> {
        let live = self.all_version_ids().await?;
        Ok(ids.iter().filter(|id| live.iter().any(|version| &version.0 == *id)).cloned().collect())
    }

    async fn live_artwork_paths(&self, ids: &[String]) -> Result<HashSet<String>, RepositoryError> {
        let state = self.state.lock().unwrap();
        Ok(state
            .artwork
            .values()
            .flatten()
            .filter(|art| ids.contains(&art.id.0))
            .flat_map(|art| art.widths.iter().map(|width| width.path.clone()))
            .collect())
    }

    async fn live_subtitle_paths(
        &self,
        ids: &[String],
    ) -> Result<HashSet<String>, RepositoryError> {
        let state = self.state.lock().unwrap();
        Ok(ids
            .iter()
            .filter_map(|id| state.subtitle_files.get(&VersionId(id.clone())))
            .flatten()
            .map(|file| file.path.clone())
            .collect())
    }

    async fn live_trickplay_paths(
        &self,
        ids: &[String],
    ) -> Result<HashSet<String>, RepositoryError> {
        let state = self.state.lock().unwrap();
        Ok(ids
            .iter()
            .filter_map(|id| state.trickplay.get(&VersionId(id.clone())))
            .flatten()
            .flat_map(|asset| asset.sheet_paths.iter().cloned())
            .collect())
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
        self.state.lock().unwrap().trickplay.insert(version.clone(), assets.to_vec());
        Ok(())
    }

    async fn set_subtitle_files(
        &self,
        version: &VersionId,
        files: &[SubtitleFile],
    ) -> Result<(), RepositoryError> {
        self.guard()?;
        if self.fail_writes.load(Ordering::Relaxed) {
            return Err(RepositoryError::Backend("mock catalog write failure".to_owned()));
        }
        self.state.lock().unwrap().subtitle_files.insert(version.clone(), files.to_vec());
        Ok(())
    }

    async fn add_subtitle_file(
        &self,
        version: &VersionId,
        file: &SubtitleFile,
    ) -> Result<(), RepositoryError> {
        self.guard()?;
        if self.fail_writes.load(Ordering::Relaxed) {
            return Err(RepositoryError::Backend("mock catalog write failure".to_owned()));
        }
        let mut state = self.state.lock().unwrap();
        let files = state.subtitle_files.entry(version.clone()).or_default();
        match files.iter_mut().find(|existing| existing.id == file.id) {
            Some(existing) => *existing = file.clone(),
            None => files.push(file.clone()),
        }
        Ok(())
    }

    async fn upsert_person(&self, person: Person) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if let Some(existing) = state.people.iter_mut().find(|p| p.id == person.id) {
            existing.name = person.name;
            if person.biography.is_some() {
                existing.biography = person.biography;
            }
            if person.birthday.is_some() {
                existing.birthday = person.birthday;
            }
            if person.deathday.is_some() {
                existing.deathday = person.deathday;
            }
            if person.place_of_birth.is_some() {
                existing.place_of_birth = person.place_of_birth;
            }
            if !person.also_known_as.is_empty() {
                existing.also_known_as = person.also_known_as;
            }
            if person.external_id.is_some() {
                existing.external_id = person.external_id;
            }
        } else {
            state.people.push(person);
        }
        Ok(())
    }

    async fn get_person(&self, id: &PersonId) -> Result<Option<Person>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        Ok(state.people.iter().find(|p| &p.id == id).cloned().map(|mut person| {
            person.artwork = state.artwork_for(&ArtworkOwner::Person(person.id.clone()));
            person
        }))
    }

    async fn set_title_enrichment(
        &self,
        owner: &TitleRef,
        enrichment: &TitleEnrichment,
    ) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state.lock().unwrap().enrichment.insert(owner.clone(), enrichment.clone());
        Ok(())
    }

    async fn movie_detail(&self, id: &MovieId) -> Result<Option<MovieDetail>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        let Some(movie) =
            state.movies.iter().find(|m| &m.id == id).cloned().map(|m| state.hydrate_movie(m))
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
        let Some(series) =
            state.series.iter().find(|s| &s.id == id).cloned().map(|s| state.hydrate_series(s))
        else {
            return Ok(None);
        };
        let enrichment = state.enrichment_for(&TitleRef::Series(id.clone()));
        let episodes: Vec<&Episode> = state
            .seasons
            .iter()
            .filter(|season| &season.series == id)
            .flat_map(|season| {
                state.episodes.iter().filter(move |episode| episode.season == season.id)
            })
            .collect();
        let playable = episodes
            .iter()
            .filter(|episode| {
                let title = TitleId::Episode(episode.id.clone());
                state.versions.iter().any(|version| version.title == title && version.available)
            })
            .count();
        Ok(Some(SeriesDetail {
            series,
            genres: enrichment.genres,
            credits: state.credited(&enrichment.credits),
            studios: enrichment.studios,
            ratings: enrichment.ratings,
            external_ids: enrichment.external_ids,
            extras: enrichment.extras,
            episodes_total: episodes.len() as u32,
            episodes_with_available_version: playable as u32,
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

    async fn list_genres(&self, kind: Option<TitleKind>) -> Result<Vec<Genre>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        let mut genres: Vec<Genre> = Vec::new();
        for (owner, enrichment) in &state.enrichment {
            if kind.is_some_and(|kind| owner.kind() != kind) {
                continue;
            }
            for genre in &enrichment.genres {
                if !genres.iter().any(|existing| existing.id == genre.id) {
                    genres.push(genre.clone());
                }
            }
        }
        genres.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.0.cmp(&b.id.0)));
        Ok(genres)
    }

    async fn list_movies_filtered(
        &self,
        filter: &TitleListFilter,
        page: PageRequest,
    ) -> Result<Page<Movie>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        let mut matched: Vec<Movie> = state
            .movies
            .iter()
            .filter(|movie| {
                genre_matches(&state, &TitleRef::Movie(movie.id.clone()), &filter.genres)
                    && !rating_blocked(movie.content_rating.as_ref(), &filter.blocked_ratings)
                    && movie_in_libraries(&state, &movie.id, filter.libraries.as_ref())
            })
            .cloned()
            .map(|m| state.hydrate_movie(m))
            .collect();
        sort_titles(&mut matched, filter.sort, filter.order);
        Ok(paginate(&matched, page))
    }

    async fn list_series_filtered(
        &self,
        filter: &TitleListFilter,
        page: PageRequest,
    ) -> Result<Page<Series>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        let mut matched: Vec<Series> = state
            .series
            .iter()
            .filter(|series| {
                genre_matches(&state, &TitleRef::Series(series.id.clone()), &filter.genres)
                    && !rating_blocked(series.content_rating.as_ref(), &filter.blocked_ratings)
                    && series_in_libraries(&state, &series.id, filter.libraries.as_ref())
            })
            .cloned()
            .map(|s| state.hydrate_series(s))
            .collect();
        sort_titles(&mut matched, filter.sort, filter.order);
        Ok(paginate(&matched, page))
    }

    async fn random_playable_title(
        &self,
        scope: &RandomScope,
        filter: &TitleListFilter,
    ) -> Result<Option<TitleId>, RepositoryError> {
        self.guard()?;
        let state = self.state.lock().unwrap();
        let members = match scope {
            RandomScope::Collection(id) => Some(
                state
                    .collections
                    .iter()
                    .find(|entry| &entry.id == id)
                    .map(|entry| entry.movies.clone())
                    .unwrap_or_default(),
            ),
            _ => None,
        };
        let found = match scope {
            RandomScope::Movies | RandomScope::Collection(_) => state
                .movies
                .iter()
                .filter(|movie| members.as_ref().is_none_or(|ids| ids.contains(&movie.id)))
                .find(|movie| movie_playable(&state, movie, filter))
                .map(|movie| TitleId::Movie(movie.id.clone())),
            RandomScope::Episodes | RandomScope::Series(_) | RandomScope::Season(_) => state
                .episodes
                .iter()
                .filter(|episode| episode_in_scope(&state, episode, scope))
                .find(|episode| episode_playable(&state, episode, filter))
                .map(|episode| TitleId::Episode(episode.id.clone())),
        };
        Ok(found)
    }
}

fn playable(state: &State, title: &TitleId, libraries: Option<&Vec<LibraryId>>) -> bool {
    state.versions.iter().any(|version| {
        &version.title == title
            && version.available
            && libraries.is_none_or(|allowed| allowed.contains(&version.library))
    })
}

fn movie_playable(state: &State, movie: &Movie, filter: &TitleListFilter) -> bool {
    genre_matches(state, &TitleRef::Movie(movie.id.clone()), &filter.genres)
        && !rating_blocked(movie.content_rating.as_ref(), &filter.blocked_ratings)
        && playable(state, &TitleId::Movie(movie.id.clone()), filter.libraries.as_ref())
}

fn episode_series<'a>(state: &'a State, episode: &Episode) -> Option<&'a Series> {
    state
        .seasons
        .iter()
        .find(|season| season.id == episode.season)
        .and_then(|season| state.series.iter().find(|entry| entry.id == season.series))
}

fn episode_in_scope(state: &State, episode: &Episode, scope: &RandomScope) -> bool {
    match scope {
        RandomScope::Season(id) => &episode.season == id,
        RandomScope::Series(id) => {
            episode_series(state, episode).is_some_and(|series| &series.id == id)
        }
        _ => true,
    }
}

fn episode_playable(state: &State, episode: &Episode, filter: &TitleListFilter) -> bool {
    let Some(series) = episode_series(state, episode) else {
        return false;
    };
    genre_matches(state, &TitleRef::Series(series.id.clone()), &filter.genres)
        && !rating_blocked(series.content_rating.as_ref(), &filter.blocked_ratings)
        && playable(state, &TitleId::Episode(episode.id.clone()), filter.libraries.as_ref())
}

fn genre_matches(state: &State, title: &TitleRef, genres: &[String]) -> bool {
    if genres.is_empty() {
        return true;
    }
    let Some(enrichment) = state.enrichment.get(title) else {
        return false;
    };
    genres.iter().all(|name| enrichment.genres.iter().any(|g| &g.name == name))
}

fn remember_artwork(state: &mut State, owner: ArtworkOwner, artwork: &[ArtworkRef]) {
    if !artwork.is_empty() {
        state.artwork.insert(owner, artwork.to_vec());
    }
}

fn split_credits(
    state: &mut State,
    owner: &TitleRef,
    credited: Vec<CreditedPerson>,
) -> Vec<Credit> {
    credited
        .into_iter()
        .map(|entry| {
            let credit = Credit {
                person: entry.person.id.clone(),
                title: owner.clone(),
                role: entry.role,
                character: entry.character,
                order: entry.order,
            };
            if !state.people.iter().any(|p| p.id == entry.person.id) {
                state.people.push(entry.person);
            }
            credit
        })
        .collect()
}

fn rating_blocked(rating: Option<&ContentRating>, blocked: &[ContentRating]) -> bool {
    let Some(rating) = rating else {
        return false;
    };
    blocked.iter().any(|entry| {
        entry.system.eq_ignore_ascii_case(&rating.system)
            && entry.code.eq_ignore_ascii_case(&rating.code)
    })
}

fn visible_contexts(state: &State, filter: &TitleListFilter) -> Vec<EpisodeContext> {
    state
        .episodes
        .iter()
        .filter_map(|episode| {
            let season = state.seasons.iter().find(|s| s.id == episode.season)?;
            let series = state.series.iter().find(|s| s.id == season.series)?;
            if rating_blocked(series.content_rating.as_ref(), &filter.blocked_ratings) {
                return None;
            }
            if !episode_in_libraries(state, &episode.id, filter.libraries.as_ref()) {
                return None;
            }
            Some(EpisodeContext {
                episode: state.hydrate_episode(episode.clone()),
                season: season.id.clone(),
                season_number: season.number,
                season_title: season.title.clone(),
                series: series.id.clone(),
            })
        })
        .collect()
}

fn episode_in_libraries(state: &State, id: &EpisodeId, libraries: Option<&Vec<LibraryId>>) -> bool {
    libraries.is_none_or(|allowed| {
        state.versions.iter().any(|version| {
            matches!(&version.title, TitleId::Episode(episode) if episode == id)
                && allowed.contains(&version.library)
        })
    })
}

fn movie_in_libraries(state: &State, id: &MovieId, libraries: Option<&Vec<LibraryId>>) -> bool {
    match libraries {
        None => true,
        Some(libraries) => state.versions.iter().any(|version| {
            matches!(&version.title, TitleId::Movie(movie) if movie == id)
                && libraries.contains(&version.library)
        }),
    }
}

fn series_in_libraries(state: &State, id: &SeriesId, libraries: Option<&Vec<LibraryId>>) -> bool {
    match libraries {
        None => true,
        Some(libraries) => state.versions.iter().any(|version| {
            if !libraries.contains(&version.library) {
                return false;
            }
            let TitleId::Episode(episode) = &version.title else {
                return false;
            };
            state
                .episodes
                .iter()
                .find(|e| &e.id == episode)
                .and_then(|e| state.seasons.iter().find(|s| s.id == e.season))
                .is_some_and(|season| &season.series == id)
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::common::Quality;
    use jiff::Timestamp;

    fn page() -> PageRequest {
        PageRequest { offset: 0, limit: 10 }
    }

    fn movie(id: &str) -> Movie {
        Movie {
            id: MovieId(id.to_owned()),
            title: id.to_owned(),
            sort_title: id.to_owned(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    fn series(id: &str) -> Series {
        Series {
            id: SeriesId(id.to_owned()),
            title: id.to_owned(),
            sort_title: id.to_owned(),
            year: None,
            overview: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
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
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
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
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
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
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
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
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        repo.add_version(version("v1"));

        assert_eq!(repo.list_movies(page()).await.unwrap().total, 1);
        assert!(repo.get_movie(&MovieId("m1".into())).await.unwrap().is_some());
        assert!(repo.get_movie(&MovieId("x".into())).await.unwrap().is_none());
        assert_eq!(repo.list_series(page()).await.unwrap().total, 1);
        assert!(repo.get_series(&SeriesId("s1".into())).await.unwrap().is_some());
        assert_eq!(repo.list_seasons(&SeriesId("s1".into())).await.unwrap().len(), 1);
        assert!(repo.get_season(&SeasonId("se1".into())).await.unwrap().is_some());
        assert_eq!(repo.list_episodes(&SeasonId("se1".into())).await.unwrap().len(), 1);
        assert!(repo.get_episode(&EpisodeId("e1".into())).await.unwrap().is_some());
        assert_eq!(repo.list_collections(page()).await.unwrap().total, 1);
        assert!(repo.get_collection(&CollectionId("c1".into())).await.unwrap().is_some());
        assert_eq!(
            repo.list_versions(&TitleId::Movie(MovieId("m1".into())), page()).await.unwrap().total,
            1
        );
        assert_eq!(
            repo.list_library_versions(&LibraryId("lib1".into()), page()).await.unwrap().total,
            1
        );
        assert!(repo.version_detail(&VersionId("v1".into())).await.unwrap().is_some());
        assert!(repo.version_detail(&VersionId("x".into())).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn set_list_and_hydrate_artwork() {
        use domain::catalog::ArtworkId;
        use domain::catalog::ArtworkWidth;
        use domain::metadata::ArtworkKind;

        let repo = MockCatalogRepo::new();
        repo.add_movie(movie("m1"));
        repo.add_movie(movie("m2"));

        let poster = ArtworkRef {
            id: ArtworkId("art-1".into()),
            kind: ArtworkKind::Poster,
            widths: vec![
                ArtworkWidth::new(180, "/art/art-1/180.jpg"),
                ArtworkWidth::new(480, "/art/art-1/480.jpg"),
            ],
        };
        let backdrop = ArtworkRef {
            id: ArtworkId("art-2".into()),
            kind: ArtworkKind::Backdrop,
            widths: vec![ArtworkWidth::new(960, "/art/art-2/960.jpg")],
        };
        repo.set_artwork(
            &ArtworkOwner::Movie(MovieId("m1".into())),
            &[poster.clone(), backdrop.clone()],
        )
        .await
        .unwrap();

        let listed = repo.list_artwork(&ArtworkOwner::Movie(MovieId("m1".into()))).await.unwrap();
        assert_eq!(listed, vec![poster.clone(), backdrop.clone()]);

        let hydrated = repo.get_movie(&MovieId("m1".into())).await.unwrap().unwrap();
        assert_eq!(hydrated.artwork, vec![poster.clone(), backdrop.clone()]);

        let listed_movies = repo.list_movies(page()).await.unwrap();
        assert_eq!(listed_movies.items[0].artwork, vec![poster.clone(), backdrop.clone()]);
        assert!(listed_movies.items[1].artwork.is_empty());

        repo.set_artwork(&ArtworkOwner::Movie(MovieId("m1".into())), std::slice::from_ref(&poster))
            .await
            .unwrap();
        let replaced = repo.get_movie(&MovieId("m1".into())).await.unwrap().unwrap();
        assert_eq!(replaced.artwork, vec![poster]);

        assert!(
            repo.list_artwork(&ArtworkOwner::Movie(MovieId("m2".into()))).await.unwrap().is_empty()
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
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        })
        .await
        .unwrap();
        repo.upsert_collection(Collection {
            id: CollectionId("c1".into()),
            name: "Renamed".into(),
            overview: None,
            movies: Vec::new(),
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        })
        .await
        .unwrap();
        let stored = repo.get_collection(&CollectionId("c1".into())).await.unwrap().unwrap();
        assert_eq!(stored.name, "Renamed");

        repo.upsert_collection(Collection {
            id: CollectionId("c2".into()),
            name: "alpha pack".into(),
            overview: None,
            movies: Vec::new(),
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        })
        .await
        .unwrap();
        let listed = repo.list_collections(PageRequest::ALL).await.unwrap();
        assert_eq!(
            listed.items.iter().map(|c| c.id.0.as_str()).collect::<Vec<_>>(),
            ["c2", "c1"],
            "collections list by name, case-insensitively, not by insertion"
        );
        repo.delete_collection(&CollectionId("c2".into())).await.unwrap();

        repo.delete_collection(&CollectionId("c1".into())).await.unwrap();
        assert!(repo.get_collection(&CollectionId("c1".into())).await.unwrap().is_none());
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
        assert!(repo.get_collection(&CollectionId("c1".into())).await.is_err());
        assert!(
            repo.upsert_collection(Collection {
                id: CollectionId("c1".into()),
                name: "x".into(),
                overview: None,
                movies: Vec::new(),
                added_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
                artwork: Vec::new(),
            })
            .await
            .is_err()
        );
        assert!(repo.delete_collection(&CollectionId("c1".into())).await.is_err());
        assert!(repo.get_season(&SeasonId("se1".into())).await.is_err());
        assert!(repo.list_versions(&TitleId::Movie(MovieId("m1".into())), page()).await.is_err());
        assert!(repo.list_library_versions(&LibraryId("lib1".into()), page()).await.is_err());
        assert!(repo.version_detail(&VersionId("v1".into())).await.is_err());
        assert!(repo.set_artwork(&ArtworkOwner::Movie(MovieId("m1".into())), &[]).await.is_err());
        assert!(repo.list_artwork(&ArtworkOwner::Movie(MovieId("m1".into()))).await.is_err());
        assert!(
            repo.set_version_tracks(&VersionId("v1".into()), &[], &[], &[], &[]).await.is_err()
        );
        assert!(repo.set_trickplay(&VersionId("v1".into()), &[]).await.is_err());
        assert!(repo.upsert_movie(movie("m1")).await.is_err());
        assert!(repo.upsert_series(series("s1")).await.is_err());
        assert!(repo.upsert_season(season("se1", "s1")).await.is_err());
        assert!(repo.upsert_episode(episode("e1", "se1")).await.is_err());
        assert!(repo.upsert_version(version("v1")).await.is_err());
        assert!(
            repo.upsert_person(Person {
                id: PersonId("p1".into()),
                name: "x".into(),
                ..Person::default()
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
        assert!(repo.list_genres(None).await.is_err());
        assert!(repo.list_movies_filtered(&TitleListFilter::default(), page()).await.is_err());
        assert!(repo.list_series_filtered(&TitleListFilter::default(), page()).await.is_err());
    }

    #[tokio::test]
    async fn filtered_lists_scope_by_library_through_versions() {
        let repo = MockCatalogRepo::new();
        repo.add_movie(movie("m1"));
        repo.add_series(series("s1"));
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

        let in_lib1 = TitleListFilter {
            libraries: Some(vec![LibraryId("lib1".into())]),
            ..TitleListFilter::default()
        };
        let movies = repo.list_movies_filtered(&in_lib1, page()).await.unwrap();
        assert_eq!(movies.total, 1);
        assert_eq!(movies.items[0].id, MovieId("m1".into()));
        assert!(repo.list_series_filtered(&in_lib1, page()).await.unwrap().items.is_empty());

        let in_lib2 = TitleListFilter {
            libraries: Some(vec![LibraryId("lib2".into())]),
            ..TitleListFilter::default()
        };
        let series = repo.list_series_filtered(&in_lib2, page()).await.unwrap();
        assert_eq!(series.total, 1);
        assert_eq!(series.items[0].id, SeriesId("s1".into()));

        let none_scope =
            TitleListFilter { libraries: Some(Vec::new()), ..TitleListFilter::default() };
        assert!(repo.list_movies_filtered(&none_scope, page()).await.unwrap().items.is_empty());
    }

    #[tokio::test]
    async fn upsert_replaces_by_id_and_is_idempotent() {
        let repo = MockCatalogRepo::new();
        repo.upsert_movie(movie("m1")).await.unwrap();
        repo.upsert_movie(Movie { title: "renamed".into(), ..movie("m1") }).await.unwrap();
        assert_eq!(repo.list_movies(page()).await.unwrap().total, 1);
        assert_eq!(repo.get_movie(&MovieId("m1".into())).await.unwrap().unwrap().title, "renamed");

        repo.upsert_series(series("s1")).await.unwrap();
        repo.upsert_series(series("s1")).await.unwrap();
        assert_eq!(repo.list_series(page()).await.unwrap().total, 1);

        repo.upsert_season(season("se1", "s1")).await.unwrap();
        repo.upsert_season(season("se1", "s1")).await.unwrap();
        assert_eq!(repo.list_seasons(&SeriesId("s1".into())).await.unwrap().len(), 1);

        repo.upsert_episode(episode("e1", "se1")).await.unwrap();
        repo.upsert_episode(episode("e1", "se1")).await.unwrap();
        assert_eq!(repo.list_episodes(&SeasonId("se1".into())).await.unwrap().len(), 1);

        repo.upsert_version(version("v1")).await.unwrap();
        repo.upsert_version(version("v1")).await.unwrap();
        assert_eq!(
            repo.list_versions(&TitleId::Movie(MovieId("m1".into())), page()).await.unwrap().total,
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
            ..Person::default()
        })
        .await
        .unwrap();
        repo.upsert_person(Person {
            id: PersonId("p2".into()),
            name: "Bob".into(),
            ..Person::default()
        })
        .await
        .unwrap();

        let movie_enrichment = TitleEnrichment {
            genres: vec![
                Genre { id: GenreId("g2".into()), name: "Drama".into() },
                Genre { id: GenreId("g1".into()), name: "Action".into() },
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
            studios: vec![Studio { id: StudioId("st1".into()), name: "Acme".into() }],
            ratings: vec![Rating { source: "tmdb".into(), value: 8.5 }],
            external_ids: vec![ExternalId { source: "tmdb".into(), value: "100".into() }],
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
                genres: vec![Genre { id: GenreId("g1".into()), name: "Action".into() }],
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

        let detail = repo.movie_detail(&MovieId("m1".into())).await.unwrap().unwrap();
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
        assert!(repo.movie_detail(&MovieId("nope".into())).await.unwrap().is_none());

        let series_detail = repo.series_detail(&SeriesId("s1".into())).await.unwrap().unwrap();
        assert_eq!(series_detail.genres.len(), 1);
        assert_eq!(series_detail.credits.len(), 1);
        assert_eq!(series_detail.credits[0].character, Some("Guest".to_owned()));
        assert!(repo.series_detail(&SeriesId("nope".into())).await.unwrap().is_none());

        let p1_film = repo.filmography(&PersonId("p1".into())).await.unwrap();
        assert_eq!(p1_film.len(), 2);
        assert_eq!(p1_film[0].title, TitleRef::Movie(MovieId("m1".into())));
        assert_eq!(p1_film[1].title, TitleRef::Series(SeriesId("s1".into())));
        let p2_film = repo.filmography(&PersonId("p2".into())).await.unwrap();
        assert_eq!(p2_film.len(), 1);
        assert_eq!(p2_film[0].role, CreditRole::Director);

        assert_eq!(repo.get_person(&PersonId("p1".into())).await.unwrap().unwrap().name, "Ada");
        assert!(repo.get_person(&PersonId("nope".into())).await.unwrap().is_none());

        let genres = repo.list_genres(None).await.unwrap();
        assert_eq!(genres.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(), ["Action", "Drama"]);

        let genre_filter = |name: &str| TitleListFilter {
            genres: vec![name.into()],
            ..TitleListFilter::default()
        };
        let by_g1 = repo.list_movies_filtered(&genre_filter("Action"), page()).await.unwrap();
        assert_eq!(by_g1.items.iter().map(|m| m.id.0.as_str()).collect::<Vec<_>>(), ["m1"]);
        assert_eq!(
            repo.list_movies_filtered(&genre_filter("Drama"), page()).await.unwrap().total,
            1
        );
        assert_eq!(
            repo.list_movies_filtered(&genre_filter("nope"), page()).await.unwrap().total,
            0
        );
        assert_eq!(
            repo.list_series_filtered(&genre_filter("Action"), page())
                .await
                .unwrap()
                .items
                .iter()
                .map(|s| s.id.0.as_str())
                .collect::<Vec<_>>(),
            ["s1"]
        );
        assert_eq!(
            repo.list_series_filtered(&genre_filter("Drama"), page()).await.unwrap().total,
            0
        );

        repo.set_title_enrichment(
            &TitleRef::Movie(MovieId("m1".into())),
            &TitleEnrichment::default(),
        )
        .await
        .unwrap();
        let cleared = repo.movie_detail(&MovieId("m1".into())).await.unwrap().unwrap();
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
            &[Chapter { title: "One".into(), start_ms: 0 }],
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
                intros: vec![IntroMarker { version: v1.clone(), start_ms: 0, end_ms: 5_000 }],
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

        repo.set_version_tracks(&v1, &[], &[], &[], &[]).await.unwrap();
        repo.set_trickplay(&v1, &[]).await.unwrap();
        let cleared = repo.version_detail(&v1).await.unwrap().unwrap();
        assert!(cleared.video.is_empty());
        assert!(cleared.trickplay.is_empty());

        assert!(repo.version_detail(&VersionId("nope".into())).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn deleting_a_version_takes_its_children_with_it() {
        use domain::media::{IntroMarker, SubtitleFileId, SubtitleFormat, SubtitleSource};

        let repo = MockCatalogRepo::new();
        repo.add_version(version("v1"));
        repo.add_version(version("v2"));
        let v1 = VersionId("v1".to_owned());

        assert_eq!(repo.get_version(&v1).await.unwrap().unwrap().path, "/media/v1.mkv");
        assert!(repo.get_version(&VersionId("nope".into())).await.unwrap().is_none());

        repo.set_version_tracks(
            &v1,
            &[],
            &[],
            &[],
            &[Chapter { title: "One".into(), start_ms: 0 }],
        )
        .await
        .unwrap();
        repo.set_subtitle_files(
            &v1,
            &[SubtitleFile {
                id: SubtitleFileId("sf1".to_owned()),
                version: v1.clone(),
                language: None,
                format: SubtitleFormat::Vtt,
                source: SubtitleSource::MachineTranslated,
                path: "/subs/v1.fr.vtt".to_owned(),
                translated_from: None,
                label: None,
                pinned: false,
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
                intros: vec![IntroMarker { version: v1.clone(), start_ms: 0, end_ms: 5_000 }],
                credits: Vec::new(),
            },
        );

        repo.delete_version(&v1).await.unwrap();

        assert!(repo.get_version(&v1).await.unwrap().is_none());
        assert!(repo.version_detail(&v1).await.unwrap().is_none());
        assert!(repo.get_version(&VersionId("v2".into())).await.unwrap().is_some());
        repo.delete_version(&v1).await.unwrap();

        repo.add_version(version("v1"));
        let reborn = repo.version_detail(&v1).await.unwrap().unwrap();
        assert!(reborn.chapters.is_empty());
        assert!(reborn.subtitle_files.is_empty());
        assert!(reborn.trickplay.is_empty());
        assert!(reborn.markers.intros.is_empty());
    }

    #[tokio::test]
    async fn adding_a_subtitle_file_appends_and_re_adding_replaces_in_place() {
        use domain::media::{SubtitleFileId, SubtitleFormat, SubtitleSource};

        let repo = MockCatalogRepo::new();
        repo.add_version(version("v1"));
        let v1 = VersionId("v1".to_owned());
        let file = |id: &str, path: &str| SubtitleFile {
            id: SubtitleFileId(id.to_owned()),
            version: v1.clone(),
            language: None,
            format: SubtitleFormat::Srt,
            source: SubtitleSource::OpenSubtitles,
            path: path.to_owned(),
            translated_from: None,
            label: None,
            pinned: false,
        };

        repo.add_subtitle_file(&v1, &file("sf1", "/subs/one.srt")).await.unwrap();
        repo.add_subtitle_file(&v1, &file("sf2", "/subs/two.srt")).await.unwrap();
        repo.add_subtitle_file(&v1, &file("sf1", "/subs/one.v2.srt")).await.unwrap();

        let files = repo.version_detail(&v1).await.unwrap().unwrap().subtitle_files;
        assert_eq!(files.iter().map(|f| f.id.0.as_str()).collect::<Vec<_>>(), vec!["sf1", "sf2"]);
        assert_eq!(files[0].path, "/subs/one.v2.srt");

        repo.set_fail_writes();
        assert!(repo.add_subtitle_file(&v1, &file("sf3", "/subs/three.srt")).await.is_err());
    }

    #[tokio::test]
    async fn a_movie_reports_every_collection_holding_it() {
        let repo = MockCatalogRepo::new();
        for (id, movies) in [
            ("c-z", vec![MovieId("m1".into())]),
            ("c-a", vec![MovieId("m1".into()), MovieId("m2".into())]),
            ("c-other", vec![MovieId("m2".into())]),
        ] {
            repo.upsert_collection(Collection {
                id: CollectionId(id.into()),
                name: id.into(),
                overview: None,
                movies,
                added_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
                artwork: Vec::new(),
            })
            .await
            .unwrap();
        }

        assert_eq!(
            repo.collections_of_movie(&MovieId("m1".into())).await.unwrap(),
            vec![CollectionId("c-a".into()), CollectionId("c-z".into())],
            "membership is reported for every collection, ordered by id"
        );
        assert!(repo.collections_of_movie(&MovieId("ghost".into())).await.unwrap().is_empty());

        repo.set_fail();
        assert!(repo.collections_of_movie(&MovieId("m1".into())).await.is_err());
    }

    #[tokio::test]
    async fn a_title_is_only_deletable_once_its_children_are_gone() {
        let repo = MockCatalogRepo::new();
        repo.add_movie(movie("m1"));
        repo.add_series(series("s1"));
        repo.add_season(season("se1", "s1"));
        repo.add_episode(episode("e1", "se1"));
        repo.add_version(version("v1"));
        let mut on_episode = version("v2");
        on_episode.title = TitleId::Episode(EpisodeId("e1".into()));
        repo.add_version(on_episode);

        let m1 = MovieId("m1".into());
        let s1 = SeriesId("s1".into());
        let se1 = SeasonId("se1".into());
        let e1 = EpisodeId("e1".into());

        assert!(!repo.delete_movie(&m1).await.unwrap());
        assert!(!repo.delete_series(&s1).await.unwrap());
        assert!(!repo.delete_season(&se1).await.unwrap());
        assert!(!repo.delete_episode(&e1).await.unwrap());
        assert!(repo.get_series(&s1).await.unwrap().is_some());

        repo.delete_version(&VersionId("v1".into())).await.unwrap();
        repo.delete_version(&VersionId("v2".into())).await.unwrap();

        assert!(repo.delete_movie(&m1).await.unwrap());
        assert!(repo.get_movie(&m1).await.unwrap().is_none());
        assert!(repo.delete_episode(&e1).await.unwrap());
        assert!(repo.delete_season(&se1).await.unwrap());
        assert!(repo.delete_series(&s1).await.unwrap());
        assert!(repo.get_series(&s1).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn deleting_a_movie_drops_it_from_every_collection() {
        let repo = MockCatalogRepo::new();
        repo.add_movie(movie("m1"));
        repo.add_collection(Collection {
            id: CollectionId("c1".to_owned()),
            name: "Saga".to_owned(),
            overview: None,
            movies: vec![MovieId("m1".to_owned()), MovieId("m2".to_owned())],
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });

        assert!(repo.delete_movie(&MovieId("m1".into())).await.unwrap());
        assert_eq!(
            repo.get_collection(&CollectionId("c1".into())).await.unwrap().unwrap().movies,
            vec![MovieId("m2".into())],
            "a deleted movie leaves no dangling membership behind"
        );
    }

    #[tokio::test]
    async fn series_detail_counts_only_episodes_with_an_available_version() {
        let repo = MockCatalogRepo::new();
        repo.add_series(series("s1"));
        repo.add_season(season("se1", "s1"));
        repo.add_episode(episode("e1", "se1"));
        repo.add_episode(episode("e2", "se1"));
        let mut playable = version("v1");
        playable.title = TitleId::Episode(EpisodeId("e1".into()));
        repo.add_version(playable);
        let mut missing = version("v2");
        missing.title = TitleId::Episode(EpisodeId("e2".into()));
        missing.available = false;
        repo.add_version(missing);

        let detail = repo.series_detail(&SeriesId("s1".into())).await.unwrap().unwrap();
        assert_eq!(detail.episodes_total, 2);
        assert_eq!(
            detail.episodes_with_available_version, 1,
            "an unavailable version does not make its episode playable"
        );
    }

    #[tokio::test]
    async fn every_delete_reports_a_backend_failure() {
        let repo = MockCatalogRepo::new();
        repo.set_fail();
        assert!(repo.delete_movie(&MovieId("m1".into())).await.is_err());
        assert!(repo.delete_series(&SeriesId("s1".into())).await.is_err());
        assert!(repo.delete_season(&SeasonId("se1".into())).await.is_err());
        assert!(repo.delete_episode(&EpisodeId("e1".into())).await.is_err());
    }
}

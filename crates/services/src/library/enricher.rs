use std::future::Future;

use domain::catalog::{
    ArtworkOwner, Collection, CollectionId, Episode, EpisodeId, Movie, MovieId, Season, SeasonId,
    Series, SeriesId, TitleId, TitleRef, Version, VersionId, in_year_order,
};
use domain::common::Quality;
use domain::error::RepositoryError;
use domain::job::JobId;
use domain::library::{
    DiscoveredFile, Library, LibraryId, LibraryKind, MatchedGroup, ParsedMedia, ResolveTarget,
    ScanReport,
};
use domain::media::{ProbeResult, SubtitleFile, SubtitleFileId, SubtitleSource};
use domain::metadata::{
    CollectionMeta, Credit, ExternalId, Genre, GenreId, MediaKind, MetadataProvider, Person,
    PersonId, Studio, StudioId, TitleEnrichment, TitleMetadata,
};
use domain::repository::{CatalogRepository, JobRepository, LibraryRepository};
use domain::text::sort_title;
use jiff::Timestamp;
use tracing::{debug, warn};
use uuid::Uuid;

use super::enrich_jobs::{EnrichmentJobs, SubtitleContext};
use super::metadata_fetch::MetadataFetcher;
use super::{
    Matcher, discover_subtitles, find_duplicates, normalize_title, parse_filename,
    quality_from_height,
};

fn resolve_quality(parsed: Option<Quality>, probe: &ProbeResult) -> Quality {
    parsed.unwrap_or_else(|| {
        let height = probe.video.iter().map(|v| v.height).max().unwrap_or(0);
        quality_from_height(height)
    })
}

const ID_NAMESPACE: Uuid = Uuid::NAMESPACE_URL;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestMode {
    Scan,
    Explicit,
}

impl IngestMode {
    fn skips_unchanged(self) -> bool {
        self == IngestMode::Scan
    }

    fn skips_existing_titles(self) -> bool {
        self == IngestMode::Scan
    }
}

#[derive(Debug, Clone, Copy)]
struct IngestPlan {
    mode: IngestMode,
    writes_title: bool,
}

impl IngestPlan {
    fn explicit() -> Self {
        Self { mode: IngestMode::Explicit, writes_title: true }
    }

    fn scanned(mode: IngestMode, title_is_known: bool) -> Self {
        Self { mode, writes_title: !title_is_known }
    }

    fn unidentified(title_is_known: bool) -> Self {
        Self { mode: IngestMode::Explicit, writes_title: !title_is_known }
    }

    fn checks_entities(&self) -> bool {
        self.writes_title && self.mode.skips_existing_titles()
    }
}

struct FileIngest<'a> {
    file: &'a DiscoveredFile,
    title: TitleId,
    library: &'a Library,
    quality: Option<Quality>,
    subtitle: &'a SubtitleContext,
    parent: Option<&'a JobId>,
    mode: IngestMode,
}

pub trait ScanEnricher {
    fn enrich(
        &self,
        library: &Library,
        report: &ScanReport,
        parent: Option<&JobId>,
    ) -> impl Future<Output = ()> + Send;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveOutcome {
    Identified,
    Unidentified,
}

pub trait ResolveIngester {
    fn ingest_resolved(
        &self,
        library: &Library,
        file: &DiscoveredFile,
        target: &ResolveTarget,
        parent: Option<&JobId>,
    ) -> impl Future<Output = Result<ResolveOutcome, RepositoryError>> + Send;

    fn ingest_fetched(
        &self,
        library: &Library,
        file: &DiscoveredFile,
        parsed: &ParsedMedia,
        parent: Option<&JobId>,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
}

pub trait MetadataRefresher {
    fn refresh(
        &self,
        title: &TitleRef,
        external_id: Option<&ExternalId>,
        force: bool,
        parent: Option<&JobId>,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
}

pub trait PersonRefresher {
    fn refresh_person(
        &self,
        id: &PersonId,
        force: bool,
        parent: Option<&JobId>,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
}

pub struct NoopEnricher;

impl ScanEnricher for NoopEnricher {
    async fn enrich(&self, _library: &Library, _report: &ScanReport, _parent: Option<&JobId>) {}
}

pub struct Enricher<C, M, J, L> {
    catalog: C,
    libraries: L,
    matcher: Matcher,
    fetch: MetadataFetcher<M>,
    enqueue: EnrichmentJobs<J>,
}

impl<C, M, J, L> Enricher<C, M, J, L> {
    pub fn new(catalog: C, provider: Option<M>, jobs: J, libraries: L) -> Self {
        Self {
            catalog,
            libraries,
            matcher: Matcher::new(),
            fetch: MetadataFetcher::new(provider),
            enqueue: EnrichmentJobs::new(jobs),
        }
    }

    pub fn with_subtitle_languages(mut self, languages: Vec<String>) -> Self {
        self.enqueue.subtitle_languages = languages;
        self
    }

    pub fn with_transcription(mut self, enabled: bool) -> Self {
        self.enqueue.transcription_enabled = enabled;
        self
    }

    pub fn with_translation_languages(mut self, languages: Vec<String>) -> Self {
        self.enqueue.translation_languages = languages;
        self
    }
}

impl<C, M, J, L> ScanEnricher for Enricher<C, M, J, L>
where
    C: CatalogRepository + Send + Sync,
    M: MetadataProvider + Send + Sync,
    J: JobRepository + Send + Sync,
    L: LibraryRepository + Send + Sync,
{
    async fn enrich(&self, library: &Library, report: &ScanReport, parent: Option<&JobId>) {
        let present: Vec<String> = report
            .discovered
            .iter()
            .map(|file| file.path.clone())
            .chain(report.skipped.iter().map(|file| file.path.clone()))
            .collect();
        let report = self.matcher.match_files(&report.discovered);
        for group in &report.matched {
            if let Err(err) = self.ingest_group(library, group, parent, IngestMode::Scan).await {
                warn!(library = %library.id.0, "scan enrichment failed: {err}");
            }
        }
        let mut detected = Vec::new();
        for duplicate in find_duplicates(&report.matched) {
            detected.push(duplicate.id.clone());
            if let Err(err) = self.libraries.insert_duplicate(&library.id, duplicate).await {
                warn!(library = %library.id.0, "persisting duplicate failed: {err}");
            }
        }
        for file in report.unmatched {
            if let Err(err) = self.libraries.insert_unmatched(file).await {
                warn!(library = %library.id.0, "persisting unmatched file failed: {err}");
            }
        }
        if let Err(err) = self.catalog.reconcile_library_versions(&library.id, &present).await {
            warn!(library = %library.id.0, "reconciling library versions failed: {err}");
        }
        if let Err(err) = self.libraries.reconcile_duplicates(&library.id, &detected).await {
            warn!(library = %library.id.0, "reconciling duplicates failed: {err}");
        }
        if let Err(err) = self.libraries.reconcile_unmatched(&library.id, &present).await {
            warn!(library = %library.id.0, "reconciling unmatched files failed: {err}");
        }
    }
}

impl<C, M, J, L> Enricher<C, M, J, L>
where
    C: CatalogRepository + Send + Sync,
    M: MetadataProvider + Send + Sync,
    J: JobRepository + Send + Sync,
    L: LibraryRepository + Send + Sync,
{
    async fn ingest_group(
        &self,
        library: &Library,
        group: &MatchedGroup,
        parent: Option<&JobId>,
        mode: IngestMode,
    ) -> Result<(), RepositoryError> {
        match library.kind {
            LibraryKind::Movie => self.ingest_movie(library, group, parent, mode).await,
            LibraryKind::Tv => self.ingest_episode(library, group, parent, mode).await,
        }
    }

    async fn ingest_movie(
        &self,
        library: &Library,
        group: &MatchedGroup,
        parent: Option<&JobId>,
        mode: IngestMode,
    ) -> Result<(), RepositoryError> {
        let known = mode.skips_existing_titles()
            && self.catalog.get_movie(&movie_id_of(&group.parsed)).await?.is_some();
        let metadata = if known {
            None
        } else {
            self.fetch
                .fetch_refresh(
                    MediaKind::Movie,
                    &group.parsed.title,
                    group.parsed.year,
                    group.parsed.external_id.as_ref(),
                )
                .await
        };
        self.write_movie(
            library,
            &group.parsed,
            &group.files,
            metadata,
            parent,
            IngestPlan::scanned(mode, known),
        )
        .await
    }

    async fn explicit_plan(
        &self,
        library: &Library,
        parsed: &ParsedMedia,
        identified: bool,
    ) -> Result<IngestPlan, RepositoryError> {
        if identified {
            return Ok(IngestPlan::explicit());
        }
        let known = match library.kind {
            LibraryKind::Movie => self.catalog.get_movie(&movie_id_of(parsed)).await?.is_some(),
            LibraryKind::Tv => match episode_ids_of(parsed) {
                Some(ids) => self.catalog.get_episode(&ids.episode).await?.is_some(),
                None => false,
            },
        };
        if known {
            warn!(
                library = %library.id.0,
                "the provider returned no metadata for [{}]; the file was linked and the stored title was kept",
                parsed.title
            );
        }
        Ok(IngestPlan::unidentified(known))
    }

    async fn write_movie(
        &self,
        library: &Library,
        parsed: &ParsedMedia,
        files: &[DiscoveredFile],
        metadata: Option<TitleMetadata>,
        parent: Option<&JobId>,
        plan: IngestPlan,
    ) -> Result<(), RepositoryError> {
        let movie_id = movie_id_of(parsed);
        let now = Timestamp::now();
        let title = display_title(metadata.as_ref(), &parsed.title);
        if plan.writes_title {
            self.catalog
                .upsert_movie(Movie {
                    id: movie_id.clone(),
                    sort_title: sort_title(&title, &library.sort_articles),
                    title,
                    year: display_year(metadata.as_ref(), parsed.year),
                    overview: metadata.as_ref().and_then(|m| m.overview.clone()),
                    runtime_minutes: metadata.as_ref().and_then(|m| m.runtime_minutes),
                    content_rating: metadata.as_ref().and_then(|m| m.content_rating.clone()),
                    manually_edited: false,
                    added_at: now,
                    updated_at: now,
                    artwork: Vec::new(),
                })
                .await?;
        }
        let subtitle = SubtitleContext {
            imdb_id: imdb_id(metadata.as_ref()),
            title: parsed.title.clone(),
            season: None,
            episode: None,
        };
        for file in files {
            self.ingest_file(FileIngest {
                file,
                title: TitleId::Movie(movie_id.clone()),
                library,
                quality: parsed.quality,
                subtitle: &subtitle,
                parent,
                mode: plan.mode,
            })
            .await?;
        }
        if !plan.writes_title {
            return Ok(());
        }
        if let Some(metadata) = &metadata {
            self.persist_enrichment(&TitleRef::Movie(movie_id.clone()), metadata, parent).await?;
            if let Some(collection) = &metadata.collection {
                self.attach_to_collection(&movie_id, collection, parent).await?;
            }
        }
        let artwork = metadata.map(|m| m.artwork).unwrap_or_default();
        self.enqueue.enqueue_artwork(ArtworkOwner::Movie(movie_id), artwork, parent).await
    }

    async fn sorted_members(&self, members: &[MovieId]) -> Result<Vec<MovieId>, RepositoryError> {
        let mut known = Vec::with_capacity(members.len());
        for id in members {
            if let Some(movie) = self.catalog.get_movie(id).await? {
                known.push(movie);
            }
        }
        Ok(in_year_order(members, &known))
    }

    async fn attach_to_collection(
        &self,
        movie: &MovieId,
        meta: &CollectionMeta,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        let id = CollectionId(derive_id(
            "collection",
            &format!("{}:{}", meta.external_id.source, meta.external_id.value),
        ));
        let now = Timestamp::now();
        let mut collection = match self.catalog.get_collection(&id).await? {
            Some(mut existing) => {
                if !existing.movies.contains(movie) {
                    existing.movies.push(movie.clone());
                }
                existing.updated_at = now;
                existing
            }
            None => Collection {
                id: id.clone(),
                name: meta.name.clone(),
                overview: None,
                movies: vec![movie.clone()],
                added_at: now,
                updated_at: now,
                artwork: Vec::new(),
            },
        };
        collection.movies = self.sorted_members(&collection.movies).await?;
        self.catalog.upsert_collection(collection).await?;
        self.enqueue
            .enqueue_artwork(ArtworkOwner::Collection(id), meta.artwork.clone(), parent)
            .await
    }

    async fn ingest_episode(
        &self,
        library: &Library,
        group: &MatchedGroup,
        parent: Option<&JobId>,
        mode: IngestMode,
    ) -> Result<(), RepositoryError> {
        let known = match episode_ids_of(&group.parsed) {
            Some(ids) => {
                mode.skips_existing_titles()
                    && self.catalog.get_episode(&ids.episode).await?.is_some()
            }
            None => false,
        };
        let metadata = if known {
            None
        } else {
            self.fetch
                .fetch_refresh(
                    MediaKind::Series,
                    &group.parsed.title,
                    group.parsed.year,
                    group.parsed.external_id.as_ref(),
                )
                .await
        };
        self.write_episode(
            library,
            &group.parsed,
            &group.files,
            metadata,
            parent,
            IngestPlan::scanned(mode, known),
        )
        .await
    }

    async fn write_episode(
        &self,
        library: &Library,
        parsed: &ParsedMedia,
        files: &[DiscoveredFile],
        metadata: Option<TitleMetadata>,
        parent: Option<&JobId>,
        plan: IngestPlan,
    ) -> Result<(), RepositoryError> {
        let Some(ids) = episode_ids_of(parsed) else {
            debug!("skipping non-episodic file in tv library");
            return Ok(());
        };
        let EpisodeIds {
            series: series_id,
            season: season_id,
            episode: episode_id,
            season_number: season_no,
            episode_number: episode_no,
        } = ids;
        let writes_series = if plan.checks_entities() {
            self.catalog.get_series(&series_id).await?.is_none()
        } else {
            plan.writes_title
        };
        let writes_season = if plan.checks_entities() {
            self.catalog.get_season(&season_id).await?.is_none()
        } else {
            plan.writes_title
        };
        let now = Timestamp::now();

        let season_info = match tmdb_external_id(metadata.as_ref()) {
            Some(external_id) => self.fetch.fetch_season(&external_id, season_no).await,
            None => None,
        };
        let season_title = season_info
            .as_ref()
            .and_then(|season| season.name.clone())
            .unwrap_or_else(|| format!("Season {season_no}"));
        let season_overview = season_info.as_ref().and_then(|season| season.overview.clone());
        let season_artwork =
            season_info.as_ref().map(|season| season.artwork.clone()).unwrap_or_default();
        let matched_episode = season_info
            .as_ref()
            .and_then(|season| season.episodes.iter().find(|ep| ep.number == episode_no));
        let episode_title = matched_episode
            .and_then(|ep| ep.name.clone())
            .unwrap_or_else(|| format!("Episode {episode_no}"));
        let episode_overview = matched_episode.and_then(|ep| ep.overview.clone());
        let episode_runtime = matched_episode.and_then(|ep| ep.runtime_minutes);
        let episode_air_date =
            matched_episode.and_then(|ep| ep.air_date.as_deref()).and_then(parse_air_date);
        let episode_artwork = matched_episode.map(|ep| ep.artwork.clone()).unwrap_or_default();

        let series_title = display_title(metadata.as_ref(), &parsed.title);
        if writes_series {
            self.catalog
                .upsert_series(Series {
                    id: series_id.clone(),
                    sort_title: sort_title(&series_title, &library.sort_articles),
                    title: series_title,
                    year: display_year(metadata.as_ref(), parsed.year),
                    overview: metadata.as_ref().and_then(|m| m.overview.clone()),
                    content_rating: metadata.as_ref().and_then(|m| m.content_rating.clone()),
                    manually_edited: false,
                    added_at: now,
                    updated_at: now,
                    artwork: Vec::new(),
                })
                .await?;
        }
        if writes_season {
            self.catalog
                .upsert_season(Season {
                    id: season_id.clone(),
                    series: series_id.clone(),
                    number: season_no,
                    title: Some(season_title),
                    overview: season_overview,
                    added_at: now,
                    updated_at: now,
                    artwork: Vec::new(),
                })
                .await?;
        }
        if plan.writes_title {
            self.catalog
                .upsert_episode(Episode {
                    id: episode_id.clone(),
                    season: season_id.clone(),
                    number: episode_no,
                    title: episode_title,
                    overview: episode_overview,
                    runtime_minutes: episode_runtime,
                    air_date: episode_air_date,
                    manually_edited: false,
                    added_at: now,
                    updated_at: now,
                    artwork: Vec::new(),
                })
                .await?;
        }
        let subtitle = SubtitleContext {
            imdb_id: imdb_id(metadata.as_ref()),
            title: parsed.title.clone(),
            season: Some(season_no),
            episode: Some(episode_no),
        };
        for file in files {
            self.ingest_file(FileIngest {
                file,
                title: TitleId::Episode(episode_id.clone()),
                library,
                quality: parsed.quality,
                subtitle: &subtitle,
                parent,
                mode: plan.mode,
            })
            .await?;
        }
        if writes_series && let Some(metadata) = &metadata {
            self.persist_enrichment(&TitleRef::Series(series_id.clone()), metadata, parent).await?;
        }
        if writes_series {
            let artwork = metadata.map(|m| m.artwork).unwrap_or_default();
            self.enqueue.enqueue_artwork(ArtworkOwner::Series(series_id), artwork, parent).await?;
        }
        if writes_season {
            self.enqueue
                .enqueue_artwork(ArtworkOwner::Season(season_id), season_artwork, parent)
                .await?;
        }
        if plan.writes_title {
            self.enqueue
                .enqueue_artwork(ArtworkOwner::Episode(episode_id), episode_artwork, parent)
                .await?;
        }
        Ok(())
    }

    async fn ingest_file(&self, ingest: FileIngest<'_>) -> Result<(), RepositoryError> {
        let FileIngest { file, title, library, quality, subtitle, parent, mode } = ingest;
        let version_id = VersionId(derive_id("version", &file.path));
        let existing = self.catalog.get_version(&version_id).await?;
        if mode.skips_unchanged() && is_unchanged(existing.as_ref(), file) {
            return Ok(());
        }
        let now = Timestamp::now();
        self.catalog
            .upsert_version(Version {
                id: version_id.clone(),
                title,
                library: library.id.clone(),
                quality: resolve_quality(quality, &file.probe),
                container: container_of(&file.path),
                path: file.path.clone(),
                size_bytes: recorded_size(file, existing.as_ref()),
                duration_ms: file.probe.duration_ms,
                available: true,
                added_at: now,
                updated_at: now,
            })
            .await?;
        self.catalog
            .set_version_tracks(
                &version_id,
                &file.probe.video,
                &file.probe.audio,
                &file.probe.subtitles,
                &file.probe.chapters,
            )
            .await?;
        let subtitle_files: Vec<SubtitleFile> =
            discover_subtitles(&file.path, &file.subtitle_siblings)
                .into_iter()
                .map(|sub| SubtitleFile {
                    id: SubtitleFileId(derive_id("subtitle", &sub.path)),
                    version: version_id.clone(),
                    language: sub.language,
                    format: sub.format,
                    source: SubtitleSource::External,
                    path: sub.path,
                    translated_from: None,
                    label: None,
                    pinned: false,
                })
                .collect();
        if !subtitle_files.is_empty() {
            self.catalog.set_subtitle_files(&version_id, &subtitle_files).await?;
        }
        let has_native_subtitle = !subtitle_files.is_empty() || !file.probe.subtitles.is_empty();
        self.enqueue.for_file(&version_id, file, subtitle, has_native_subtitle, parent).await
    }

    async fn persist_enrichment(
        &self,
        owner: &TitleRef,
        metadata: &TitleMetadata,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        let mut credits = Vec::new();
        let mut people = Vec::new();
        for info in &metadata.cast {
            let person = PersonId(derive_id("person", &info.external_person_id.value));
            self.catalog
                .upsert_person(Person {
                    id: person.clone(),
                    name: info.name.clone(),
                    external_id: Some(info.external_person_id.value.clone()),
                    ..Person::default()
                })
                .await?;
            if info.external_person_id.source == "tmdb" {
                people.push(person.clone());
            }
            credits.push(Credit {
                person,
                title: owner.clone(),
                role: info.role,
                character: info.character.clone(),
                order: info.order,
            });
        }
        let genres = metadata
            .genres
            .iter()
            .map(|name| Genre { id: GenreId(derive_id("genre", name)), name: name.clone() })
            .collect();
        let studios = metadata
            .studios
            .iter()
            .map(|name| Studio { id: StudioId(derive_id("studio", name)), name: name.clone() })
            .collect();
        let enrichment = TitleEnrichment {
            genres,
            credits,
            studios,
            ratings: metadata.ratings.clone(),
            external_ids: metadata.external_ids.clone(),
            extras: Vec::new(),
        };
        self.catalog.set_title_enrichment(owner, &enrichment).await?;
        self.enqueue.enqueue_person_metadata(people, parent).await
    }

    async fn version_library(&self, title: &TitleId) -> Option<LibraryId> {
        super::version_library(&self.catalog, title).await
    }

    async fn series_library(&self, series: &SeriesId) -> Option<LibraryId> {
        super::series_library(&self.catalog, series).await
    }

    async fn library_articles(&self, library: Option<LibraryId>) -> Vec<String>
    where
        L: LibraryRepository + Send + Sync,
    {
        super::library_articles(&self.libraries, library).await
    }

    async fn stored_movie_id(&self, id: &MovieId) -> Option<ExternalId> {
        let detail = self.catalog.movie_detail(id).await.ok()??;
        detail.external_ids.into_iter().find(|external| external.source == "tmdb")
    }

    async fn stored_series_id(&self, id: &SeriesId) -> Option<ExternalId> {
        let detail = self.catalog.series_detail(id).await.ok()??;
        detail.external_ids.into_iter().find(|external| external.source == "tmdb")
    }

    async fn refresh_movie(
        &self,
        id: &MovieId,
        external_id: Option<&ExternalId>,
        force: bool,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        let Some(existing) = self.catalog.get_movie(id).await? else {
            debug!("refresh skipped for missing movie {}", id.0);
            return Ok(());
        };
        if force && existing.manually_edited {
            self.catalog.upsert_movie(Movie { manually_edited: false, ..existing.clone() }).await?;
        }
        let stored = match external_id {
            Some(_) => None,
            None => self.stored_movie_id(id).await,
        };
        let Some(metadata) = self
            .fetch
            .fetch_refresh(
                MediaKind::Movie,
                &existing.title,
                existing.year,
                external_id.or(stored.as_ref()),
            )
            .await
        else {
            debug!("refresh found no metadata for movie {}", id.0);
            return Ok(());
        };
        if existing.manually_edited && !force {
            debug!("refresh kept manual edits for movie {}", id.0);
        } else {
            let mut updated = existing.clone();
            if !metadata.title.trim().is_empty() {
                updated.title = metadata.title.clone();
            }
            let library = self.version_library(&TitleId::Movie(id.clone())).await;
            let articles = self.library_articles(library).await;
            updated.sort_title = sort_title(&updated.title, &articles);
            updated.year = metadata.year.or(existing.year);
            updated.overview = metadata.overview.clone().or(existing.overview);
            updated.runtime_minutes = metadata.runtime_minutes.or(existing.runtime_minutes);
            updated.content_rating = metadata.content_rating.clone().or(existing.content_rating);
            updated.manually_edited = false;
            self.catalog.upsert_movie(updated).await?;
        }
        self.persist_enrichment(&TitleRef::Movie(id.clone()), &metadata, parent).await?;
        if let Some(collection) = &metadata.collection {
            self.attach_to_collection(id, collection, parent).await?;
        }
        self.enqueue
            .enqueue_artwork(ArtworkOwner::Movie(id.clone()), metadata.artwork, parent)
            .await
    }

    async fn discard_series_edits(
        &self,
        id: &SeriesId,
        series: &Series,
    ) -> Result<(), RepositoryError> {
        if series.manually_edited {
            self.catalog.upsert_series(Series { manually_edited: false, ..series.clone() }).await?;
        }
        for season in self.catalog.list_seasons(id).await? {
            for episode in self.catalog.list_episodes(&season.id).await? {
                if episode.manually_edited {
                    self.catalog
                        .upsert_episode(Episode { manually_edited: false, ..episode })
                        .await?;
                }
            }
        }
        Ok(())
    }

    async fn refresh_series(
        &self,
        id: &SeriesId,
        external_id: Option<&ExternalId>,
        force: bool,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        let Some(existing) = self.catalog.get_series(id).await? else {
            debug!("refresh skipped for missing series {}", id.0);
            return Ok(());
        };
        if force {
            self.discard_series_edits(id, &existing).await?;
        }
        let stored = match external_id {
            Some(_) => None,
            None => self.stored_series_id(id).await,
        };
        let Some(metadata) = self
            .fetch
            .fetch_refresh(
                MediaKind::Series,
                &existing.title,
                existing.year,
                external_id.or(stored.as_ref()),
            )
            .await
        else {
            debug!("refresh found no metadata for series {}", id.0);
            return Ok(());
        };
        if existing.manually_edited && !force {
            debug!("refresh kept manual edits for series {}", id.0);
        } else {
            let mut updated = existing.clone();
            if !metadata.title.trim().is_empty() {
                updated.title = metadata.title.clone();
            }
            let library = self.series_library(id).await;
            let articles = self.library_articles(library).await;
            updated.sort_title = sort_title(&updated.title, &articles);
            updated.year = metadata.year.or(existing.year);
            updated.overview = metadata.overview.clone().or(existing.overview);
            updated.content_rating = metadata.content_rating.clone().or(existing.content_rating);
            updated.manually_edited = false;
            self.catalog.upsert_series(updated).await?;
        }
        self.persist_enrichment(&TitleRef::Series(id.clone()), &metadata, parent).await?;
        let series_ref = tmdb_external_id(Some(&metadata));
        self.enqueue
            .enqueue_artwork(ArtworkOwner::Series(id.clone()), metadata.artwork, parent)
            .await?;
        if let Some(series_ref) = series_ref {
            self.refresh_seasons(id, &series_ref, force, parent).await?;
        }
        Ok(())
    }

    async fn refresh_seasons(
        &self,
        series: &SeriesId,
        series_ref: &ExternalId,
        force: bool,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        for season in self.catalog.list_seasons(series).await? {
            let Some(info) = self.fetch.fetch_season(series_ref, season.number).await else {
                continue;
            };
            let now = Timestamp::now();
            let mut updated = season.clone();
            if info.name.is_some() {
                updated.title = info.name.clone();
            }
            if info.overview.is_some() {
                updated.overview = info.overview.clone();
            }
            updated.updated_at = now;
            self.catalog.upsert_season(updated).await?;
            self.enqueue
                .enqueue_artwork(ArtworkOwner::Season(season.id.clone()), info.artwork, parent)
                .await?;
            for episode in self.catalog.list_episodes(&season.id).await? {
                let Some(ep) = info.episodes.iter().find(|ep| ep.number == episode.number) else {
                    continue;
                };
                if episode.manually_edited && !force {
                    debug!("refresh kept manual edits for episode {}", episode.id.0);
                } else {
                    let mut updated = episode.clone();
                    if let Some(name) = ep.name.clone() {
                        updated.title = name;
                    }
                    if ep.overview.is_some() {
                        updated.overview = ep.overview.clone();
                    }
                    if ep.runtime_minutes.is_some() {
                        updated.runtime_minutes = ep.runtime_minutes;
                    }
                    if let Some(air_date) = ep.air_date.as_deref().and_then(parse_air_date) {
                        updated.air_date = Some(air_date);
                    }
                    updated.manually_edited = false;
                    updated.updated_at = now;
                    self.catalog.upsert_episode(updated).await?;
                }
                self.enqueue
                    .enqueue_artwork(
                        ArtworkOwner::Episode(episode.id.clone()),
                        ep.artwork.clone(),
                        parent,
                    )
                    .await?;
            }
        }
        Ok(())
    }
}

impl<C, M, J, L> ResolveIngester for Enricher<C, M, J, L>
where
    C: CatalogRepository + Send + Sync,
    M: MetadataProvider + Send + Sync,
    J: JobRepository + Send + Sync,
    L: LibraryRepository + Send + Sync,
{
    async fn ingest_resolved(
        &self,
        library: &Library,
        file: &DiscoveredFile,
        target: &ResolveTarget,
        parent: Option<&JobId>,
    ) -> Result<ResolveOutcome, RepositoryError> {
        let parsed = parse_filename(&file.path);
        match target {
            ResolveTarget::Existing(title) => {
                let subtitle = SubtitleContext {
                    imdb_id: None,
                    title: parsed.title.clone(),
                    season: parsed.season,
                    episode: parsed.episode,
                };
                self.ingest_file(FileIngest {
                    file,
                    title: title.clone(),
                    library,
                    quality: parsed.quality,
                    subtitle: &subtitle,
                    parent,
                    mode: IngestMode::Explicit,
                })
                .await?;
                Ok(ResolveOutcome::Identified)
            }
            ResolveTarget::Provider(external_id) => {
                let external_id = qualified_provider_id(external_id, library.kind);
                let metadata = self.fetch.fetch_by_id(&external_id).await;
                let identified = metadata.is_some();
                let plan = self.explicit_plan(library, &parsed, identified).await?;
                let files = std::slice::from_ref(file);
                match library.kind {
                    LibraryKind::Movie => {
                        self.write_movie(library, &parsed, files, metadata, parent, plan).await?;
                    }
                    LibraryKind::Tv => {
                        self.write_episode(library, &parsed, files, metadata, parent, plan).await?;
                    }
                }
                Ok(if identified {
                    ResolveOutcome::Identified
                } else {
                    ResolveOutcome::Unidentified
                })
            }
        }
    }

    async fn ingest_fetched(
        &self,
        library: &Library,
        file: &DiscoveredFile,
        parsed: &ParsedMedia,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        let metadata = match parsed.external_id.as_ref() {
            Some(id) => self.fetch.fetch_by_id(id).await,
            None => None,
        };
        let plan = self.explicit_plan(library, parsed, metadata.is_some()).await?;
        let files = std::slice::from_ref(file);
        match library.kind {
            LibraryKind::Movie => {
                self.write_movie(library, parsed, files, metadata, parent, plan).await
            }
            LibraryKind::Tv => {
                self.write_episode(library, parsed, files, metadata, parent, plan).await
            }
        }
    }
}

impl<C, M, J, L> MetadataRefresher for Enricher<C, M, J, L>
where
    C: CatalogRepository + Send + Sync,
    M: MetadataProvider + Send + Sync,
    J: JobRepository + Send + Sync,
    L: LibraryRepository + Send + Sync,
{
    async fn refresh(
        &self,
        title: &TitleRef,
        external_id: Option<&ExternalId>,
        force: bool,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        match title {
            TitleRef::Movie(id) => self.refresh_movie(id, external_id, force, parent).await,
            TitleRef::Series(id) => self.refresh_series(id, external_id, force, parent).await,
        }
    }
}

impl<C, M, J, L> PersonRefresher for Enricher<C, M, J, L>
where
    C: CatalogRepository + Send + Sync,
    M: MetadataProvider + Send + Sync,
    J: JobRepository + Send + Sync,
    L: Send + Sync,
{
    async fn refresh_person(
        &self,
        id: &PersonId,
        force: bool,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        let Some(person) = self.catalog.get_person(id).await? else {
            debug!("person refresh skipped for missing person {}", id.0);
            return Ok(());
        };
        if !force && person.biography.is_some() {
            return Ok(());
        }
        let Some(value) = person.external_id.clone() else {
            return Ok(());
        };
        let external_id = ExternalId { source: "tmdb".to_owned(), value };
        let Some(meta) = self.fetch.fetch_person(&external_id).await else {
            return Ok(());
        };
        let mut updated = person.clone();
        if !meta.name.is_empty() {
            updated.name = meta.name;
        }
        if meta.biography.is_some() {
            updated.biography = meta.biography;
        }
        if meta.birthday.is_some() {
            updated.birthday = meta.birthday;
        }
        if meta.deathday.is_some() {
            updated.deathday = meta.deathday;
        }
        if meta.place_of_birth.is_some() {
            updated.place_of_birth = meta.place_of_birth;
        }
        if !meta.also_known_as.is_empty() {
            updated.also_known_as = meta.also_known_as;
        }
        self.catalog.upsert_person(updated).await?;
        self.enqueue.enqueue_artwork(ArtworkOwner::Person(id.clone()), meta.artwork, parent).await
    }
}

pub fn derive_id(kind: &str, key: &str) -> String {
    Uuid::new_v5(&ID_NAMESPACE, format!("{kind}:{key}").as_bytes()).to_string()
}

pub fn container_of(path: &str) -> String {
    let base = path.rsplit(['/', '\\']).next().unwrap_or(path);
    base.rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .unwrap_or_else(|| "bin".to_owned())
}

fn qualified_provider_id(id: &ExternalId, kind: LibraryKind) -> ExternalId {
    if id.source != "tmdb" || id.value.contains('/') || id.value.starts_with("tt") {
        return id.clone();
    }
    let prefix = match kind {
        LibraryKind::Movie => "movie",
        LibraryKind::Tv => "tv",
    };
    ExternalId { source: id.source.clone(), value: format!("{prefix}/{}", id.value) }
}

fn movie_id_of(parsed: &ParsedMedia) -> MovieId {
    let slug = normalize_title(&parsed.title);
    MovieId(derive_id("movie", &format!("{slug}:{}", parsed.year.unwrap_or(0))))
}

struct EpisodeIds {
    series: SeriesId,
    season: SeasonId,
    episode: EpisodeId,
    season_number: u16,
    episode_number: u16,
}

fn episode_ids_of(parsed: &ParsedMedia) -> Option<EpisodeIds> {
    let (season_number, episode_number) = (parsed.season?, parsed.episode?);
    let series_key = normalize_title(&parsed.title);
    Some(EpisodeIds {
        series: SeriesId(derive_id("series", &series_key)),
        season: SeasonId(derive_id("season", &format!("{series_key}:{season_number}"))),
        episode: EpisodeId(derive_id(
            "episode",
            &format!("{series_key}:{season_number}:{episode_number}"),
        )),
        season_number,
        episode_number,
    })
}

fn recorded_size(file: &DiscoveredFile, existing: Option<&Version>) -> u64 {
    match (file.size_bytes, existing) {
        (0, Some(version)) => version.size_bytes,
        (size, _) => size,
    }
}

fn is_unchanged(existing: Option<&Version>, file: &DiscoveredFile) -> bool {
    existing.is_some_and(|version| {
        version.size_bytes == recorded_size(file, existing)
            && version.duration_ms == file.probe.duration_ms
    })
}

fn imdb_id(metadata: Option<&TitleMetadata>) -> Option<String> {
    metadata?.external_ids.iter().find(|id| id.source == "imdb").map(|id| id.value.clone())
}

fn tmdb_external_id(metadata: Option<&TitleMetadata>) -> Option<ExternalId> {
    metadata?.external_ids.iter().find(|id| id.source == "tmdb").cloned()
}

fn display_title(metadata: Option<&TitleMetadata>, parsed: &str) -> String {
    metadata
        .map(|m| m.title.trim())
        .filter(|title| !title.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| parsed.to_owned())
}

fn display_year(metadata: Option<&TitleMetadata>, parsed: Option<u16>) -> Option<u16> {
    metadata.and_then(|m| m.year).or(parsed)
}

fn parse_air_date(value: &str) -> Option<Timestamp> {
    let date: jiff::civil::Date = value.parse().ok()?;
    date.to_zoned(jiff::tz::TimeZone::UTC).ok().map(|zoned| zoned.timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::{
        ArtworkJobPayload, MetadataJobPayload, SubtitleJobPayload, TranscriptionJobPayload,
        TrickplayJobPayload,
    };
    use domain::error::MetadataError;
    use domain::job::JobKind;
    use domain::library::{
        DiscoveredFile, LibraryId, LibraryOrigin, ResolutionStatus, WatcherStrategy,
    };
    use domain::media::ProbeResult;
    use domain::metadata::{
        Artwork, ArtworkKind, ContentRating, CreditInfo, CreditRole, EpisodeArtwork, ExternalId,
        MetadataMatch, MetadataQuery, PersonMetadata, Rating, SeasonArtwork, TitleMetadata,
    };
    use mocks::{MockCatalogRepo, MockJobStore, MockLibraryRepo, MockMetadataProvider};

    #[test]
    fn resolve_quality_prefers_filename_then_falls_back_to_probed_height() {
        use domain::media::VideoTrack;
        let track = |height: u32| VideoTrack {
            index: 0,
            codec: "h264".to_owned(),
            width: height * 16 / 9,
            height,
            bit_depth: 8,
            hdr: None,
            frame_rate: 24.0,
            bitrate: None,
        };
        let uhd = ProbeResult {
            duration_ms: 0,
            video: vec![track(2160)],
            audio: Vec::new(),
            subtitles: Vec::new(),
            chapters: Vec::new(),
        };
        let no_video = ProbeResult {
            duration_ms: 0,
            video: Vec::new(),
            audio: Vec::new(),
            subtitles: Vec::new(),
            chapters: Vec::new(),
        };
        assert_eq!(resolve_quality(Some(Quality::Hd), &uhd), Quality::Hd);
        assert_eq!(resolve_quality(None, &uhd), Quality::Uhd);
        assert_eq!(resolve_quality(None, &no_video), Quality::Sd);
    }

    enum ProviderMode {
        SearchErr,
        Empty,
        FetchErr,
        Artwork(Vec<Artwork>),
        Full(Box<TitleMetadata>),
        ByIdOnly(Box<TitleMetadata>),
        Season(Box<TitleMetadata>, SeasonArtwork),
        PersonInfo(PersonMetadata),
    }

    struct MockProvider {
        mode: ProviderMode,
    }

    impl MetadataProvider for MockProvider {
        async fn search(&self, query: &MetadataQuery) -> Result<Vec<MetadataMatch>, MetadataError> {
            match self.mode {
                ProviderMode::SearchErr => Err(MetadataError::Backend("boom".into())),
                ProviderMode::Empty | ProviderMode::ByIdOnly(_) => Ok(Vec::new()),
                _ => Ok(vec![MetadataMatch {
                    external_id: ExternalId { source: "tmdb".into(), value: "movie/1".into() },
                    title: "match".into(),
                    year: query.year,
                    kind: MediaKind::Movie,
                }]),
            }
        }

        async fn fetch(&self, _id: &ExternalId) -> Result<TitleMetadata, MetadataError> {
            match &self.mode {
                ProviderMode::Artwork(artwork) => {
                    Ok(TitleMetadata { artwork: artwork.clone(), ..TitleMetadata::default() })
                }
                ProviderMode::Full(metadata) => Ok((**metadata).clone()),
                ProviderMode::ByIdOnly(metadata) => Ok((**metadata).clone()),
                ProviderMode::Season(metadata, _) => Ok((**metadata).clone()),
                _ => Err(MetadataError::NotFound),
            }
        }

        async fn fetch_season(
            &self,
            _id: &ExternalId,
            _season: u16,
        ) -> Result<SeasonArtwork, MetadataError> {
            match &self.mode {
                ProviderMode::Season(_, season) => Ok(season.clone()),
                _ => Err(MetadataError::NotFound),
            }
        }

        async fn fetch_person(&self, _id: &ExternalId) -> Result<PersonMetadata, MetadataError> {
            match &self.mode {
                ProviderMode::PersonInfo(person) => Ok(person.clone()),
                _ => Err(MetadataError::NotFound),
            }
        }
    }

    fn art(kind: ArtworkKind) -> Artwork {
        Artwork {
            kind,
            language: None,
            source: "tmdb".into(),
            url: format!("https://cdn/{kind:?}.jpg"),
        }
    }

    fn library(kind: LibraryKind) -> Library {
        Library {
            id: LibraryId("lib".into()),
            name: "Lib".into(),
            origin: LibraryOrigin::Local,
            kind,
            sort_articles: vec!["the".into()],
            roots: vec!["/m".into()],
            watcher: WatcherStrategy::Manual,
            scan_schedule: None,
            metadata_sources: Vec::new(),
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn discovered(path: &str) -> DiscoveredFile {
        DiscoveredFile {
            library: LibraryId("lib".into()),
            path: path.into(),
            size_bytes: 10,
            subtitle_siblings: Vec::new(),
            probe: ProbeResult {
                duration_ms: 1000,
                video: Vec::new(),
                audio: Vec::new(),
                subtitles: Vec::new(),
                chapters: Vec::new(),
            },
        }
    }

    fn report(paths: &[&str]) -> ScanReport {
        ScanReport {
            discovered: paths.iter().map(|p| discovered(p)).collect(),
            skipped: Vec::new(),
            total_candidates: paths.len(),
        }
    }

    fn enricher(
        catalog: MockCatalogRepo,
        provider: Option<MockProvider>,
        jobs: MockJobStore,
    ) -> Enricher<MockCatalogRepo, MockProvider, MockJobStore, MockLibraryRepo> {
        Enricher::new(catalog, provider, jobs, MockLibraryRepo::new())
    }

    fn enricher_with_libraries(
        catalog: MockCatalogRepo,
        provider: Option<MockProvider>,
        jobs: MockJobStore,
        libraries: MockLibraryRepo,
    ) -> Enricher<MockCatalogRepo, MockProvider, MockJobStore, MockLibraryRepo> {
        Enricher::new(catalog, provider, jobs, libraries)
    }

    fn page() -> domain::common::PageRequest {
        domain::common::PageRequest { offset: 0, limit: 10 }
    }

    async fn count_kind(jobs: &MockJobStore, kind: JobKind) -> usize {
        jobs.list().await.unwrap().iter().filter(|j| j.kind == kind).count()
    }

    // Reconciling is what drops rows the scan no longer sees. A failure there leaves stale
    // rows behind, which is bad, but abandoning the scan halfway leaves worse, so each one
    // is reported and the next is still attempted.
    #[tracing_test::traced_test]
    #[tokio::test]
    async fn a_library_store_that_cannot_reconcile_still_finishes_the_scan() {
        let catalog = MockCatalogRepo::new();
        let libraries = MockLibraryRepo::new();
        let svc = enricher_with_libraries(catalog, None, MockJobStore::new(), libraries.clone());
        libraries.set_fail_reconcile();

        svc.enrich(&library(LibraryKind::Movie), &report(&["/m/The Matrix (1999).mkv"]), None)
            .await;

        assert!(logs_contain("reconciling duplicates failed"));
        assert!(logs_contain("reconciling unmatched files failed"));
    }

    #[tokio::test]
    async fn movie_ingest_without_provider_writes_rows_and_enqueues_trickplay() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let svc = enricher(catalog.clone(), None, jobs.clone());

        svc.enrich(
            &library(LibraryKind::Movie),
            &report(&["/m/The Matrix (1999) 1080p.mkv"]),
            None,
        )
        .await;

        let movies = catalog.list_movies(page()).await.unwrap();
        assert_eq!(movies.total, 1);
        assert_eq!(movies.items[0].title, "The Matrix");
        assert_eq!(movies.items[0].year, Some(1999));
        assert!(movies.items[0].overview.is_none());
        assert!(movies.items[0].runtime_minutes.is_none());
        assert!(movies.items[0].content_rating.is_none());
        let detail = catalog.movie_detail(&movies.items[0].id).await.unwrap().unwrap();
        assert!(detail.genres.is_empty());
        assert!(detail.credits.is_empty());
        assert!(detail.studios.is_empty());
        assert_eq!(
            catalog.list_library_versions(&LibraryId("lib".into()), page()).await.unwrap().total,
            1
        );
        assert_eq!(count_kind(&jobs, JobKind::Artwork).await, 0);
        assert_eq!(count_kind(&jobs, JobKind::Trickplay).await, 1);
        assert_eq!(count_kind(&jobs, JobKind::Subtitles).await, 0);
    }

    #[tokio::test]
    async fn movie_ingest_enqueues_subtitles_when_languages_configured() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let svc = enricher(catalog.clone(), None, jobs.clone())
            .with_subtitle_languages(vec!["en".into()]);

        svc.enrich(
            &library(LibraryKind::Movie),
            &report(&["/m/The Matrix (1999) 1080p.mkv"]),
            None,
        )
        .await;

        assert_eq!(count_kind(&jobs, JobKind::Subtitles).await, 1);
    }

    #[tokio::test]
    async fn enqueued_jobs_carry_the_parent_job_id() {
        let jobs = MockJobStore::new();
        let svc = enricher(MockCatalogRepo::new(), None, jobs.clone())
            .with_subtitle_languages(vec!["en".into()])
            .with_translation_languages(vec!["fr".into()]);
        let parent = JobId("scan-1".into());

        svc.enrich(
            &library(LibraryKind::Movie),
            &audio_scan(
                "/m/The Matrix (1999) 1080p.mkv",
                vec!["/m/The Matrix (1999) 1080p.en.srt".into()],
                Vec::new(),
            ),
            Some(&parent),
        )
        .await;

        let all = jobs.list().await.unwrap();
        assert!(all.iter().any(|j| j.kind == JobKind::Trickplay));
        assert!(all.iter().any(|j| j.kind == JobKind::Translation));
        assert!(!all.is_empty());
        assert!(all.iter().all(|j| j.parent_id.as_ref() == Some(&parent)));
    }

    fn audio_scan(
        path: &str,
        siblings: Vec<String>,
        subtitles: Vec<domain::media::EmbeddedSubtitleTrack>,
    ) -> ScanReport {
        ScanReport {
            discovered: vec![DiscoveredFile {
                library: LibraryId("lib".into()),
                path: path.into(),
                size_bytes: 10,
                subtitle_siblings: siblings,
                probe: ProbeResult {
                    duration_ms: 1000,
                    video: Vec::new(),
                    audio: vec![domain::media::AudioTrack {
                        index: 1,
                        codec: "aac".into(),
                        channels: 2,
                        language: None,
                        bitrate: None,
                    }],
                    subtitles,
                    chapters: Vec::new(),
                },
            }],
            skipped: Vec::new(),
            total_candidates: 1,
        }
    }

    fn embedded_subtitle() -> domain::media::EmbeddedSubtitleTrack {
        domain::media::EmbeddedSubtitleTrack {
            index: 2,
            language: None,
            format: domain::media::SubtitleFormat::Srt,
            forced: false,
            default: true,
        }
    }

    #[tokio::test]
    async fn transcription_enqueued_when_enabled_no_subs_and_audio() {
        let jobs = MockJobStore::new();
        let svc = enricher(MockCatalogRepo::new(), None, jobs.clone()).with_transcription(true);

        svc.enrich(
            &library(LibraryKind::Movie),
            &audio_scan("/m/The Matrix (1999) 1080p.mkv", Vec::new(), Vec::new()),
            None,
        )
        .await;

        assert_eq!(count_kind(&jobs, JobKind::Transcription).await, 1);
        let job = jobs
            .list()
            .await
            .unwrap()
            .into_iter()
            .find(|j| j.kind == JobKind::Transcription)
            .unwrap();
        let payload = TranscriptionJobPayload::decode(&job.payload).unwrap();
        assert_eq!(payload.audio_track_index, None);
        assert_eq!(payload.source_language, None);
    }

    #[tokio::test]
    async fn transcription_deferred_to_subtitles_when_opensubtitles_configured() {
        let jobs = MockJobStore::new();
        let svc = enricher(MockCatalogRepo::new(), None, jobs.clone())
            .with_transcription(true)
            .with_subtitle_languages(vec!["spa".into()]);

        let mut scan = audio_scan("/m/The Matrix (1999) 1080p.mkv", Vec::new(), Vec::new());
        scan.discovered[0].probe.audio = vec![
            domain::media::AudioTrack {
                index: 1,
                codec: "eac3".into(),
                channels: 6,
                language: Some(domain::common::LanguageCode("eng".into())),
                bitrate: None,
            },
            domain::media::AudioTrack {
                index: 2,
                codec: "aac".into(),
                channels: 2,
                language: Some(domain::common::LanguageCode("spa".into())),
                bitrate: None,
            },
        ];

        svc.enrich(&library(LibraryKind::Movie), &scan, None).await;

        assert_eq!(count_kind(&jobs, JobKind::Transcription).await, 0);
        let subtitles =
            jobs.list().await.unwrap().into_iter().find(|j| j.kind == JobKind::Subtitles).unwrap();
        let payload = SubtitleJobPayload::decode(&subtitles.payload).unwrap();
        assert!(payload.transcribe_on_miss);
    }

    #[tokio::test]
    async fn transcription_labels_from_first_audio_track_language() {
        let jobs = MockJobStore::new();
        let svc = enricher(MockCatalogRepo::new(), None, jobs.clone()).with_transcription(true);

        let mut scan = audio_scan("/m/The Matrix (1999) 1080p.mkv", Vec::new(), Vec::new());
        scan.discovered[0].probe.audio = vec![domain::media::AudioTrack {
            index: 1,
            codec: "eac3".into(),
            channels: 6,
            language: Some(domain::common::LanguageCode("eng".into())),
            bitrate: None,
        }];

        svc.enrich(&library(LibraryKind::Movie), &scan, None).await;

        let job = jobs
            .list()
            .await
            .unwrap()
            .into_iter()
            .find(|j| j.kind == JobKind::Transcription)
            .unwrap();
        let payload = TranscriptionJobPayload::decode(&job.payload).unwrap();
        assert_eq!(payload.audio_track_index, None);
        assert_eq!(payload.source_language.as_deref(), Some("eng"));
    }

    #[tokio::test]
    async fn transcription_not_enqueued_when_disabled() {
        let jobs = MockJobStore::new();
        let svc = enricher(MockCatalogRepo::new(), None, jobs.clone());

        svc.enrich(
            &library(LibraryKind::Movie),
            &audio_scan("/m/The Matrix (1999) 1080p.mkv", Vec::new(), Vec::new()),
            None,
        )
        .await;

        assert_eq!(count_kind(&jobs, JobKind::Transcription).await, 0);
    }

    #[tokio::test]
    async fn transcription_not_enqueued_when_sidecar_subtitle_present() {
        let jobs = MockJobStore::new();
        let svc = enricher(MockCatalogRepo::new(), None, jobs.clone()).with_transcription(true);

        svc.enrich(
            &library(LibraryKind::Movie),
            &audio_scan(
                "/m/The Matrix (1999) 1080p.mkv",
                vec!["/m/The Matrix (1999) 1080p.en.srt".into()],
                Vec::new(),
            ),
            None,
        )
        .await;

        assert_eq!(count_kind(&jobs, JobKind::Transcription).await, 0);
    }

    #[tokio::test]
    async fn transcription_not_enqueued_when_embedded_subtitle_present() {
        let jobs = MockJobStore::new();
        let svc = enricher(MockCatalogRepo::new(), None, jobs.clone()).with_transcription(true);

        svc.enrich(
            &library(LibraryKind::Movie),
            &audio_scan("/m/The Matrix (1999) 1080p.mkv", Vec::new(), vec![embedded_subtitle()]),
            None,
        )
        .await;

        assert_eq!(count_kind(&jobs, JobKind::Transcription).await, 0);
    }

    #[tokio::test]
    async fn transcription_not_enqueued_without_audio() {
        let jobs = MockJobStore::new();
        let svc = enricher(MockCatalogRepo::new(), None, jobs.clone()).with_transcription(true);

        svc.enrich(
            &library(LibraryKind::Movie),
            &report(&["/m/The Matrix (1999) 1080p.mkv"]),
            None,
        )
        .await;

        assert_eq!(count_kind(&jobs, JobKind::Transcription).await, 0);
    }

    #[tokio::test]
    async fn translation_enqueued_when_native_subtitle_and_languages() {
        let jobs = MockJobStore::new();
        let svc = enricher(MockCatalogRepo::new(), None, jobs.clone())
            .with_translation_languages(vec!["fr".into()]);

        svc.enrich(
            &library(LibraryKind::Movie),
            &audio_scan(
                "/m/The Matrix (1999) 1080p.mkv",
                vec!["/m/The Matrix (1999) 1080p.en.srt".into()],
                Vec::new(),
            ),
            None,
        )
        .await;

        assert_eq!(count_kind(&jobs, JobKind::Translation).await, 1);
    }

    #[tokio::test]
    async fn translation_not_enqueued_without_native_subtitle() {
        let jobs = MockJobStore::new();
        let svc = enricher(MockCatalogRepo::new(), None, jobs.clone())
            .with_translation_languages(vec!["fr".into()]);

        svc.enrich(
            &library(LibraryKind::Movie),
            &audio_scan("/m/The Matrix (1999) 1080p.mkv", Vec::new(), Vec::new()),
            None,
        )
        .await;

        assert_eq!(count_kind(&jobs, JobKind::Translation).await, 0);
    }

    #[tokio::test]
    async fn translation_not_enqueued_without_languages() {
        let jobs = MockJobStore::new();
        let svc = enricher(MockCatalogRepo::new(), None, jobs.clone());

        svc.enrich(
            &library(LibraryKind::Movie),
            &audio_scan(
                "/m/The Matrix (1999) 1080p.mkv",
                vec!["/m/The Matrix (1999) 1080p.en.srt".into()],
                Vec::new(),
            ),
            None,
        )
        .await;

        assert_eq!(count_kind(&jobs, JobKind::Translation).await, 0);
    }

    #[test]
    fn imdb_id_extracts_matching_source() {
        let metadata = TitleMetadata {
            external_ids: vec![
                ExternalId { source: "tmdb".into(), value: "movie/603".into() },
                ExternalId { source: "imdb".into(), value: "tt0133093".into() },
            ],
            ..TitleMetadata::default()
        };
        assert_eq!(imdb_id(Some(&metadata)), Some("tt0133093".to_owned()));
        assert_eq!(imdb_id(Some(&TitleMetadata::default())), None);
        assert_eq!(imdb_id(None), None);
    }

    #[tokio::test]
    async fn movie_ingest_applies_metadata_and_persists_enrichment() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let metadata = TitleMetadata {
            title: "The Matrix".into(),
            year: Some(1999),
            overview: Some("Neo learns the truth.".into()),
            runtime_minutes: Some(136),
            content_rating: Some(ContentRating { system: "MPAA".into(), code: "R".into() }),
            ratings: vec![Rating { source: "tmdb".into(), value: 8.5 }],
            genres: vec!["Action".into(), "Sci-Fi".into()],
            cast: vec![
                CreditInfo {
                    external_person_id: ExternalId {
                        source: "tmdb".into(),
                        value: "person/1".into(),
                    },
                    name: "Keanu Reeves".into(),
                    role: CreditRole::Actor,
                    character: Some("Neo".into()),
                    order: 0,
                },
                CreditInfo {
                    external_person_id: ExternalId {
                        source: "tmdb".into(),
                        value: "person/2".into(),
                    },
                    name: "Lana Wachowski".into(),
                    role: CreditRole::Director,
                    character: None,
                    order: 0,
                },
            ],
            studios: vec!["Warner Bros.".into()],
            artwork: vec![art(ArtworkKind::Poster)],
            external_ids: vec![ExternalId { source: "tmdb".into(), value: "movie/603".into() }],
            collection: None,
        };
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Full(Box::new(metadata)) }),
            jobs.clone(),
        );

        svc.enrich(
            &library(LibraryKind::Movie),
            &report(&["/m/The Matrix (1999) 1080p.mkv"]),
            None,
        )
        .await;

        let movie = catalog.list_movies(page()).await.unwrap().items[0].clone();
        assert_eq!(movie.overview.as_deref(), Some("Neo learns the truth."));
        assert_eq!(movie.runtime_minutes, Some(136));
        assert_eq!(movie.content_rating.as_ref().unwrap().code, "R");

        let detail = catalog.movie_detail(&movie.id).await.unwrap().unwrap();
        assert_eq!(detail.genres.len(), 2);
        assert_eq!(detail.credits.len(), 2);
        assert_eq!(detail.credits[0].person.name, "Keanu Reeves");
        assert_eq!(detail.credits[0].character.as_deref(), Some("Neo"));
        assert_eq!(detail.credits[1].role, CreditRole::Director);
        assert_eq!(detail.studios.len(), 1);
        assert_eq!(detail.studios[0].name, "Warner Bros.");
        assert_eq!(detail.ratings.len(), 1);
        assert_eq!(detail.external_ids.len(), 1);

        let person =
            catalog.get_person(&PersonId(derive_id("person", "person/1"))).await.unwrap().unwrap();
        assert_eq!(person.name, "Keanu Reeves");

        assert_eq!(count_kind(&jobs, JobKind::Artwork).await, 1);
        assert_eq!(count_kind(&jobs, JobKind::Trickplay).await, 1);
    }

    fn tmdb_collection(name: &str) -> CollectionMeta {
        CollectionMeta {
            external_id: ExternalId { source: "tmdb".into(), value: "collection/2344".into() },
            name: name.into(),
            artwork: vec![art(ArtworkKind::Poster), art(ArtworkKind::Backdrop)],
        }
    }

    fn movie_with_collection(name: &str) -> TitleMetadata {
        TitleMetadata { collection: Some(tmdb_collection(name)), ..TitleMetadata::default() }
    }

    fn matrix_collection_id() -> CollectionId {
        CollectionId(derive_id("collection", "tmdb:collection/2344"))
    }

    #[tokio::test]
    async fn movie_ingest_attaches_to_collection() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(movie_with_collection("The Matrix Collection"))),
            }),
            jobs.clone(),
        );

        svc.enrich(
            &library(LibraryKind::Movie),
            &report(&["/m/The Matrix (1999) 1080p.mkv"]),
            None,
        )
        .await;

        let movie = catalog.list_movies(page()).await.unwrap().items[0].clone();
        let collection = catalog
            .get_collection(&matrix_collection_id())
            .await
            .unwrap()
            .expect("collection created");
        assert_eq!(collection.name, "The Matrix Collection");
        assert_eq!(collection.movies, vec![movie.id]);

        assert_eq!(count_kind(&jobs, JobKind::Artwork).await, 1);
        let artwork =
            jobs.list().await.unwrap().into_iter().find(|j| j.kind == JobKind::Artwork).unwrap();
        let payload = ArtworkJobPayload::decode(&artwork.payload).unwrap();
        assert!(matches!(payload.owner, ArtworkOwner::Collection(_)));
        assert_eq!(payload.items.len(), 2);
    }

    #[tokio::test]
    async fn second_movie_joins_existing_collection() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(movie_with_collection("Matrix"))),
            }),
            jobs.clone(),
        );

        svc.enrich(
            &library(LibraryKind::Movie),
            &report(&["/m/The Matrix (1999) 1080p.mkv", "/m/The Matrix Reloaded (2003) 1080p.mkv"]),
            None,
        )
        .await;

        let collection = catalog
            .get_collection(&matrix_collection_id())
            .await
            .unwrap()
            .expect("collection created");
        assert_eq!(collection.movies.len(), 2);
        let movies = catalog.list_movies(page()).await.unwrap();
        assert_eq!(movies.total, 2);
    }

    #[tokio::test]
    async fn a_collection_is_stored_in_release_order_whatever_the_scan_order() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(movie_with_collection("Matrix"))),
            }),
            jobs.clone(),
        );

        // Scanned newest first, which is what left the Up next rail out of order.
        svc.enrich(
            &library(LibraryKind::Movie),
            &report(&["/m/The Matrix Reloaded (2003) 1080p.mkv", "/m/The Matrix (1999) 1080p.mkv"]),
            None,
        )
        .await;

        let collection = catalog
            .get_collection(&matrix_collection_id())
            .await
            .unwrap()
            .expect("collection created");
        let movies = catalog.list_movies(page()).await.unwrap().items;
        let years: Vec<Option<u16>> = collection
            .movies
            .iter()
            .map(|id| movies.iter().find(|m| &m.id == id).and_then(|m| m.year))
            .collect();
        assert_eq!(years, vec![Some(1999), Some(2003)]);
    }

    #[tokio::test]
    async fn re_ingesting_same_movie_keeps_single_membership() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(movie_with_collection("Matrix"))),
            }),
            jobs.clone(),
        );
        let report = report(&["/m/The Matrix (1999) 1080p.mkv"]);

        svc.enrich(&library(LibraryKind::Movie), &report, None).await;
        svc.enrich(&library(LibraryKind::Movie), &report, None).await;

        let collection = catalog
            .get_collection(&matrix_collection_id())
            .await
            .unwrap()
            .expect("collection created");
        assert_eq!(collection.movies.len(), 1);
    }

    #[tokio::test]
    async fn rescanning_an_existing_movie_never_consults_the_provider() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let report = report(&["/m/The Matrix (1999) 1080p.mkv"]);
        let first = enricher(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(TitleMetadata {
                    title: "The Matrix".into(),
                    overview: Some("A hacker learns the truth.".into()),
                    artwork: vec![art(ArtworkKind::Poster)],
                    ..TitleMetadata::default()
                })),
            }),
            jobs.clone(),
        );

        first.enrich(&library(LibraryKind::Movie), &report, None).await;
        let artwork_after_first = count_kind(&jobs, JobKind::Artwork).await;

        let second = enricher(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(TitleMetadata {
                    title: "Something Else Entirely".into(),
                    artwork: vec![art(ArtworkKind::Backdrop)],
                    ..TitleMetadata::default()
                })),
            }),
            jobs.clone(),
        );
        second.enrich(&library(LibraryKind::Movie), &report, None).await;

        assert_eq!(artwork_after_first, 1);
        assert_eq!(count_kind(&jobs, JobKind::Artwork).await, 1);
        let movie = catalog
            .get_movie(&MovieId(derive_id("movie", "the matrix:1999")))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(movie.title, "The Matrix");
        assert_eq!(movie.overview.as_deref(), Some("A hacker learns the truth."));
    }

    #[tokio::test]
    async fn a_new_episode_of_a_known_series_only_writes_the_episode() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let metadata = TitleMetadata {
            title: "Gamma".into(),
            external_ids: vec![ExternalId { source: "tmdb".into(), value: "tv/1399".into() }],
            artwork: vec![art(ArtworkKind::Poster)],
            ..TitleMetadata::default()
        };
        let season = SeasonArtwork {
            number: 1,
            name: Some("First Season".into()),
            artwork: vec![art(ArtworkKind::Poster)],
            episodes: vec![
                EpisodeArtwork {
                    number: 1,
                    artwork: vec![art(ArtworkKind::Backdrop)],
                    ..EpisodeArtwork::default()
                },
                EpisodeArtwork {
                    number: 2,
                    artwork: vec![art(ArtworkKind::Backdrop)],
                    ..EpisodeArtwork::default()
                },
            ],
            ..SeasonArtwork::default()
        };
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Season(Box::new(metadata), season) }),
            jobs.clone(),
        );
        let library = library(LibraryKind::Tv);

        svc.enrich(&library, &report(&["/tv/Gamma S01E01 720p.mkv"]), None).await;
        let after_first = count_kind(&jobs, JobKind::Artwork).await;

        svc.enrich(
            &library,
            &report(&["/tv/Gamma S01E01 720p.mkv", "/tv/Gamma S01E02 720p.mkv"]),
            None,
        )
        .await;

        assert_eq!(after_first, 3);
        assert_eq!(count_kind(&jobs, JobKind::Artwork).await, 4);
        let owners: Vec<ArtworkOwner> = jobs
            .list()
            .await
            .unwrap()
            .iter()
            .filter(|j| j.kind == JobKind::Artwork)
            .map(|j| ArtworkJobPayload::decode(&j.payload).unwrap().owner)
            .collect();
        let series_key = normalize_title("Gamma");
        let second_episode = EpisodeId(derive_id("episode", &format!("{series_key}:1:2")));
        assert_eq!(
            owners.iter().filter(|owner| matches!(owner, ArtworkOwner::Series(_))).count(),
            1
        );
        assert_eq!(
            owners.iter().filter(|owner| matches!(owner, ArtworkOwner::Season(_))).count(),
            1
        );
        assert!(owners.contains(&ArtworkOwner::Episode(second_episode.clone())));
        assert!(catalog.get_episode(&second_episode).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn rescanning_an_unchanged_version_enqueues_no_further_work() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let svc = enricher(catalog.clone(), None, jobs.clone());
        let report = report(&["/m/The Matrix (1999) 1080p.mkv"]);

        svc.enrich(&library(LibraryKind::Movie), &report, None).await;
        let after_first = count_kind(&jobs, JobKind::Trickplay).await;
        svc.enrich(&library(LibraryKind::Movie), &report, None).await;

        assert_eq!(after_first, 1);
        assert_eq!(count_kind(&jobs, JobKind::Trickplay).await, 1);
    }

    #[tokio::test]
    async fn rescanning_keeps_subtitles_generated_since_the_last_scan() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let svc = enricher(catalog.clone(), None, jobs.clone());
        let path = "/m/The Matrix (1999) 1080p.mkv";
        let report = report(&[path]);
        let version = VersionId(derive_id("version", path));

        svc.enrich(&library(LibraryKind::Movie), &report, None).await;
        catalog
            .set_subtitle_files(
                &version,
                &[SubtitleFile {
                    id: SubtitleFileId("translated".into()),
                    version: version.clone(),
                    language: Some(domain::common::LanguageCode("fr".into())),
                    format: domain::media::SubtitleFormat::Vtt,
                    source: SubtitleSource::MachineTranslated,
                    path: "/subs/matrix.fr.vtt".into(),
                    translated_from: None,
                    label: None,
                    pinned: false,
                }],
            )
            .await
            .unwrap();

        svc.enrich(&library(LibraryKind::Movie), &report, None).await;

        let detail = catalog.version_detail(&version).await.unwrap().unwrap();
        assert_eq!(detail.subtitle_files.len(), 1);
        assert_eq!(detail.subtitle_files[0].source, SubtitleSource::MachineTranslated);
    }

    #[tokio::test]
    async fn a_file_that_changed_is_ingested_again() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let svc = enricher(catalog.clone(), None, jobs.clone());
        let path = "/m/The Matrix (1999) 1080p.mkv";
        let library = library(LibraryKind::Movie);

        svc.enrich(&library, &report(&[path]), None).await;

        let mut resized = discovered(path);
        resized.size_bytes = 99;
        svc.enrich(
            &library,
            &ScanReport { discovered: vec![resized], skipped: Vec::new(), total_candidates: 1 },
            None,
        )
        .await;
        assert_eq!(count_kind(&jobs, JobKind::Trickplay).await, 2);

        let mut restretched = discovered(path);
        restretched.probe.duration_ms = 4321;
        restretched.size_bytes = 99;
        svc.enrich(
            &library,
            &ScanReport { discovered: vec![restretched], skipped: Vec::new(), total_candidates: 1 },
            None,
        )
        .await;

        assert_eq!(count_kind(&jobs, JobKind::Trickplay).await, 3);
        let version =
            catalog.get_version(&VersionId(derive_id("version", path))).await.unwrap().unwrap();
        assert_eq!(version.size_bytes, 99);
        assert_eq!(version.duration_ms, 4321);
    }

    #[tokio::test]
    async fn a_relink_re_ingests_an_unchanged_version_and_keeps_its_size() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let svc = enricher(catalog.clone(), None, jobs.clone());
        let path = "/m/The Matrix (1999) 1080p.mkv";
        let library = library(LibraryKind::Movie);
        svc.enrich(&library, &report(&[path]), None).await;

        let mut unstatted = discovered(path);
        unstatted.size_bytes = 0;
        svc.ingest_resolved(
            &library,
            &unstatted,
            &ResolveTarget::Existing(TitleId::Movie(MovieId("other".into()))),
            None,
        )
        .await
        .unwrap();

        let version =
            catalog.get_version(&VersionId(derive_id("version", path))).await.unwrap().unwrap();
        assert_eq!(version.title, TitleId::Movie(MovieId("other".into())));
        assert_eq!(version.size_bytes, 10);
    }

    #[tokio::test]
    async fn a_relink_to_an_existing_title_survives_the_next_scan() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let svc = enricher(catalog.clone(), None, jobs.clone());
        let path = "/m/HDR Sample (2020) 1080p.mkv";
        let library = library(LibraryKind::Movie);
        let report = report(&[path]);
        svc.enrich(&library, &report, None).await;

        catalog.add_movie(Movie {
            id: MovieId("m-matrix".into()),
            title: "The Matrix".into(),
            sort_title: "matrix, the".into(),
            year: Some(1999),
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        svc.ingest_resolved(
            &library,
            &discovered(path),
            &ResolveTarget::Existing(TitleId::Movie(MovieId("m-matrix".into()))),
            None,
        )
        .await
        .unwrap();

        svc.enrich(&library, &report, None).await;

        let version =
            catalog.get_version(&VersionId(derive_id("version", path))).await.unwrap().unwrap();
        assert_eq!(version.title, TitleId::Movie(MovieId("m-matrix".into())));
    }

    #[tokio::test]
    async fn a_provider_relink_keeps_its_name_through_the_next_scan() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let path = "/m/HDR Sample (2020) 1080p.mkv";
        let library = library(LibraryKind::Movie);
        let report = report(&[path]);
        let scan = enricher(catalog.clone(), None, jobs.clone());
        scan.enrich(&library, &report, None).await;

        let relink = enricher(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(TitleMetadata {
                    title: "The Matrix".into(),
                    ..TitleMetadata::default()
                })),
            }),
            jobs.clone(),
        );
        relink
            .ingest_resolved(
                &library,
                &discovered(path),
                &ResolveTarget::Provider(ExternalId {
                    source: "tmdb".into(),
                    value: "movie/603".into(),
                }),
                None,
            )
            .await
            .unwrap();

        scan.enrich(&library, &report, None).await;

        let movie = catalog
            .get_movie(&MovieId(derive_id("movie", "hdr sample:2020")))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(movie.title, "The Matrix");
    }

    #[tokio::test]
    async fn a_metadata_refresh_keeps_its_name_through_the_next_scan() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let path = "/m/HDR Sample (2020) 1080p.mkv";
        let library = library(LibraryKind::Movie);
        let report = report(&[path]);
        let scan = enricher(catalog.clone(), None, jobs.clone());
        scan.enrich(&library, &report, None).await;

        let id = MovieId(derive_id("movie", "hdr sample:2020"));
        let refresh = enricher(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(renaming_metadata("The Matrix"))),
            }),
            jobs.clone(),
        );
        refresh.refresh(&TitleRef::Movie(id.clone()), None, false, None).await.unwrap();

        scan.enrich(&library, &report, None).await;

        let movie = catalog.get_movie(&id).await.unwrap().unwrap();
        assert_eq!(movie.title, "The Matrix");
    }

    #[tokio::test]
    async fn the_provider_year_wins_over_the_one_in_the_filename() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(TitleMetadata {
                    title: "Blade Runner".into(),
                    year: Some(1982),
                    ..TitleMetadata::default()
                })),
            }),
            jobs.clone(),
        );

        svc.enrich(
            &library(LibraryKind::Movie),
            &report(&["/m/Blade Runner (1981) 1080p.mkv"]),
            None,
        )
        .await;

        let movie = catalog.list_movies(page()).await.unwrap().items[0].clone();
        assert_eq!(movie.year, Some(1982));
        assert_eq!(movie.id, MovieId(derive_id("movie", "blade runner:1981")));
    }

    #[tokio::test]
    async fn a_refresh_adopts_the_provider_year() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let scan = enricher(catalog.clone(), None, jobs.clone());
        scan.enrich(
            &library(LibraryKind::Movie),
            &report(&["/m/HDR Sample (2020) 1080p.mkv"]),
            None,
        )
        .await;

        let id = MovieId(derive_id("movie", "hdr sample:2020"));
        assert_eq!(catalog.get_movie(&id).await.unwrap().unwrap().year, Some(2020));

        let refresh = enricher(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(TitleMetadata {
                    year: Some(2018),
                    ..refresh_metadata()
                })),
            }),
            jobs.clone(),
        );
        refresh.refresh(&TitleRef::Movie(id.clone()), None, false, None).await.unwrap();

        assert_eq!(catalog.get_movie(&id).await.unwrap().unwrap().year, Some(2018));
    }

    #[tokio::test]
    async fn existing_collection_name_and_overview_are_preserved() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        catalog
            .upsert_collection(Collection {
                id: matrix_collection_id(),
                name: "My Curated Set".into(),
                overview: Some("hand written".into()),
                movies: Vec::new(),
                added_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
                artwork: Vec::new(),
            })
            .await
            .unwrap();
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(movie_with_collection("TMDB Name"))),
            }),
            jobs.clone(),
        );

        svc.enrich(
            &library(LibraryKind::Movie),
            &report(&["/m/The Matrix (1999) 1080p.mkv"]),
            None,
        )
        .await;

        let collection = catalog.get_collection(&matrix_collection_id()).await.unwrap().unwrap();
        assert_eq!(collection.name, "My Curated Set");
        assert_eq!(collection.overview.as_deref(), Some("hand written"));
        assert_eq!(collection.movies.len(), 1);
    }

    #[tokio::test]
    async fn movie_ingest_persists_probe_tracks_and_trickplay_payload() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let svc = enricher(catalog.clone(), None, jobs.clone());
        let path = "/m/The Matrix (1999) 1080p.mkv";
        let file = DiscoveredFile {
            library: LibraryId("lib".into()),
            path: path.into(),
            size_bytes: 10,
            subtitle_siblings: Vec::new(),
            probe: ProbeResult {
                duration_ms: 7_200_000,
                video: Vec::new(),
                audio: vec![domain::media::AudioTrack {
                    index: 1,
                    codec: "aac".into(),
                    channels: 6,
                    language: None,
                    bitrate: None,
                }],
                subtitles: Vec::new(),
                chapters: Vec::new(),
            },
        };
        let scan = ScanReport { discovered: vec![file], skipped: Vec::new(), total_candidates: 1 };

        svc.enrich(&library(LibraryKind::Movie), &scan, None).await;

        let version_id = VersionId(derive_id("version", path));
        let detail = catalog.version_detail(&version_id).await.unwrap().unwrap();
        assert_eq!(detail.audio.len(), 1);
        assert_eq!(detail.audio[0].channels, 6);

        let enqueued = jobs.list().await.unwrap();
        let trickplay = enqueued.iter().find(|j| j.kind == JobKind::Trickplay).unwrap();
        let payload = TrickplayJobPayload::decode(&trickplay.payload).unwrap();
        assert_eq!(payload.version_id, version_id);
        assert_eq!(payload.source_path, path);
        assert_eq!(payload.duration_ms, 7_200_000);
    }

    #[tokio::test]
    async fn movie_ingest_persists_sidecar_subtitles() {
        let catalog = MockCatalogRepo::new();
        let svc = enricher(catalog.clone(), None, MockJobStore::new());
        let path = "/m/The Matrix (1999) 1080p.mkv";
        let file = DiscoveredFile {
            library: LibraryId("lib".into()),
            path: path.into(),
            size_bytes: 10,
            subtitle_siblings: vec![
                "/m/The Matrix (1999) 1080p.en.srt".into(),
                "/m/The Matrix (1999) 1080p.fr.srt".into(),
            ],
            probe: ProbeResult {
                duration_ms: 7_200_000,
                video: Vec::new(),
                audio: Vec::new(),
                subtitles: Vec::new(),
                chapters: Vec::new(),
            },
        };
        let scan = ScanReport { discovered: vec![file], skipped: Vec::new(), total_candidates: 1 };

        svc.enrich(&library(LibraryKind::Movie), &scan, None).await;

        let version_id = VersionId(derive_id("version", path));
        let detail = catalog.version_detail(&version_id).await.unwrap().unwrap();
        assert_eq!(detail.subtitle_files.len(), 2);
        assert!(detail.subtitle_files.iter().all(|s| matches!(s.source, SubtitleSource::External)));
        let langs: Vec<_> = detail
            .subtitle_files
            .iter()
            .filter_map(|s| s.language.as_ref().map(|l| l.0.clone()))
            .collect();
        assert!(langs.contains(&"en".to_owned()));
        assert!(langs.contains(&"fr".to_owned()));
    }

    #[tokio::test]
    async fn movie_ingest_is_idempotent_across_rescans() {
        let catalog = MockCatalogRepo::new();
        let svc = enricher(catalog.clone(), None, MockJobStore::new());
        let lib = library(LibraryKind::Movie);
        let scan = report(&["/m/The Matrix (1999) 1080p.mkv"]);

        svc.enrich(&lib, &scan, None).await;
        svc.enrich(&lib, &scan, None).await;

        assert_eq!(catalog.list_movies(page()).await.unwrap().total, 1);
        assert_eq!(
            catalog.list_library_versions(&LibraryId("lib".into()), page()).await.unwrap().total,
            1
        );
    }

    #[tokio::test]
    async fn rescan_marks_vanished_version_unavailable_but_keeps_the_title() {
        let catalog = MockCatalogRepo::new();
        let svc = enricher(catalog.clone(), None, MockJobStore::new());
        let lib = library(LibraryKind::Movie);

        svc.enrich(
            &lib,
            &report(&["/m/The Matrix (1999) 1080p.mkv", "/m/Alien (1979) 1080p.mkv"]),
            None,
        )
        .await;
        let seeded = catalog.list_library_versions(&LibraryId("lib".into()), page()).await.unwrap();
        assert_eq!(seeded.total, 2);
        assert!(seeded.items.iter().all(|v| v.available));

        svc.enrich(&lib, &report(&["/m/The Matrix (1999) 1080p.mkv"]), None).await;

        let after = catalog.list_library_versions(&LibraryId("lib".into()), page()).await.unwrap();
        assert_eq!(after.total, 2);
        assert_eq!(catalog.list_movies(page()).await.unwrap().total, 2);
        let matrix =
            after.items.iter().find(|v| v.path == "/m/The Matrix (1999) 1080p.mkv").unwrap();
        let alien = after.items.iter().find(|v| v.path == "/m/Alien (1979) 1080p.mkv").unwrap();
        assert!(matrix.available);
        assert!(!alien.available);
    }

    #[tokio::test]
    async fn tv_ingest_writes_tree_and_enqueues_series_artwork() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let provider = MockProvider {
            mode: ProviderMode::Artwork(vec![
                art(ArtworkKind::Poster),
                art(ArtworkKind::Backdrop),
                art(ArtworkKind::Banner),
            ]),
        };
        let svc = enricher(catalog.clone(), Some(provider), jobs.clone());

        svc.enrich(&library(LibraryKind::Tv), &report(&["/tv/Gamma S01E01 720p.mkv"]), None).await;

        let series = catalog.list_series(page()).await.unwrap();
        assert_eq!(series.total, 1);
        assert_eq!(series.items[0].title, "Gamma");
        let series_id = series.items[0].id.clone();
        assert_eq!(catalog.list_seasons(&series_id).await.unwrap().len(), 1);
        let season_id = catalog.list_seasons(&series_id).await.unwrap()[0].id.clone();
        assert_eq!(catalog.list_episodes(&season_id).await.unwrap().len(), 1);
        assert_eq!(
            catalog.list_library_versions(&LibraryId("lib".into()), page()).await.unwrap().total,
            1
        );

        assert_eq!(count_kind(&jobs, JobKind::Trickplay).await, 1);
        let enqueued = jobs.list().await.unwrap();
        let artwork = enqueued.iter().find(|j| j.kind == JobKind::Artwork).unwrap();
        let payload = ArtworkJobPayload::decode(&artwork.payload).unwrap();
        assert!(matches!(payload.owner, ArtworkOwner::Series(_)));
        assert_eq!(payload.items.len(), 2);
        assert!(
            payload
                .items
                .iter()
                .all(|i| matches!(i.kind, ArtworkKind::Poster | ArtworkKind::Backdrop))
        );
    }

    #[tokio::test]
    async fn tv_ingest_enqueues_season_and_episode_artwork() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let metadata = TitleMetadata {
            external_ids: vec![ExternalId { source: "tmdb".into(), value: "tv/1399".into() }],
            ..TitleMetadata::default()
        };
        let season = SeasonArtwork {
            number: 1,
            name: Some("First Season".into()),
            artwork: vec![art(ArtworkKind::Poster)],
            episodes: vec![
                EpisodeArtwork {
                    number: 1,
                    name: Some("Pilot".into()),
                    overview: Some("It begins.".into()),
                    air_date: Some("2010-10-01".into()),
                    runtime_minutes: Some(42),
                    artwork: vec![art(ArtworkKind::Backdrop)],
                },
                EpisodeArtwork {
                    number: 2,
                    artwork: vec![art(ArtworkKind::Backdrop)],
                    ..EpisodeArtwork::default()
                },
            ],
            ..SeasonArtwork::default()
        };
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Season(Box::new(metadata), season) }),
            jobs.clone(),
        );

        svc.enrich(&library(LibraryKind::Tv), &report(&["/tv/Gamma S01E01 720p.mkv"]), None).await;

        let artwork_jobs: Vec<ArtworkJobPayload> = jobs
            .list()
            .await
            .unwrap()
            .iter()
            .filter(|j| j.kind == JobKind::Artwork)
            .map(|j| ArtworkJobPayload::decode(&j.payload).unwrap())
            .collect();

        let season_job = artwork_jobs
            .iter()
            .find(|p| matches!(p.owner, ArtworkOwner::Season(_)))
            .expect("season artwork enqueued");
        assert_eq!(season_job.items.len(), 1);
        assert_eq!(season_job.items[0].kind, ArtworkKind::Poster);

        let episode_job = artwork_jobs
            .iter()
            .find(|p| matches!(p.owner, ArtworkOwner::Episode(_)))
            .expect("episode artwork enqueued");
        assert_eq!(episode_job.items.len(), 1);
        assert_eq!(episode_job.items[0].kind, ArtworkKind::Backdrop);

        let series_id = catalog.list_series(page()).await.unwrap().items[0].id.clone();
        let season = catalog.list_seasons(&series_id).await.unwrap()[0].clone();
        assert_eq!(season.title.as_deref(), Some("First Season"));
        let episode = catalog.list_episodes(&season.id).await.unwrap()[0].clone();
        assert_eq!(episode.title, "Pilot");
        assert_eq!(episode.overview.as_deref(), Some("It begins."));
        assert_eq!(episode.runtime_minutes, Some(42));
        assert!(episode.air_date.is_some());
    }

    #[tokio::test]
    async fn tv_ingest_without_season_metadata_enqueues_only_series_artwork() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let metadata = TitleMetadata {
            artwork: vec![art(ArtworkKind::Poster)],
            external_ids: vec![ExternalId { source: "tmdb".into(), value: "tv/1399".into() }],
            ..TitleMetadata::default()
        };
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Full(Box::new(metadata)) }),
            jobs.clone(),
        );

        svc.enrich(&library(LibraryKind::Tv), &report(&["/tv/Gamma S01E01 720p.mkv"]), None).await;

        let owners: Vec<ArtworkOwner> = jobs
            .list()
            .await
            .unwrap()
            .iter()
            .filter(|j| j.kind == JobKind::Artwork)
            .map(|j| ArtworkJobPayload::decode(&j.payload).unwrap().owner)
            .collect();
        assert!(owners.iter().any(|o| matches!(o, ArtworkOwner::Series(_))));
        assert!(!owners.iter().any(|o| matches!(o, ArtworkOwner::Season(_))));
        assert!(!owners.iter().any(|o| matches!(o, ArtworkOwner::Episode(_))));
    }

    #[tokio::test]
    async fn movie_ingest_enqueues_person_metadata_for_cast() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let metadata = TitleMetadata {
            cast: vec![CreditInfo {
                external_person_id: ExternalId { source: "tmdb".into(), value: "person/1".into() },
                name: "Keanu Reeves".into(),
                role: CreditRole::Actor,
                character: None,
                order: 0,
            }],
            external_ids: vec![ExternalId { source: "tmdb".into(), value: "movie/603".into() }],
            ..TitleMetadata::default()
        };
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Full(Box::new(metadata)) }),
            jobs.clone(),
        );

        svc.enrich(
            &library(LibraryKind::Movie),
            &report(&["/m/The Matrix (1999) 1080p.mkv"]),
            None,
        )
        .await;

        let job = jobs
            .list()
            .await
            .unwrap()
            .into_iter()
            .find(|j| j.kind == JobKind::Metadata)
            .expect("person metadata job enqueued");
        match MetadataJobPayload::decode(&job.payload).unwrap() {
            MetadataJobPayload::People { ids, force } => {
                assert!(!force);
                assert_eq!(ids.len(), 1);
            }
            other => panic!("expected people payload, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn refresh_person_fetches_and_enqueues_photo() {
        let catalog = MockCatalogRepo::new();
        catalog
            .upsert_person(Person {
                id: PersonId("p1".into()),
                name: "Ada".into(),
                external_id: Some("person/1".into()),
                ..Person::default()
            })
            .await
            .unwrap();
        let jobs = MockJobStore::new();
        let meta = PersonMetadata {
            name: "Ada Lovelace".into(),
            biography: Some("A mathematician.".into()),
            birthday: Some("1815-12-10".into()),
            deathday: Some("1852-11-27".into()),
            place_of_birth: Some("London".into()),
            also_known_as: vec!["Augusta Ada King".into()],
            artwork: vec![art(ArtworkKind::Poster)],
        };
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::PersonInfo(meta) }),
            jobs.clone(),
        );

        svc.refresh_person(&PersonId("p1".into()), false, None).await.unwrap();

        let person = catalog.get_person(&PersonId("p1".into())).await.unwrap().unwrap();
        assert_eq!(person.name, "Ada Lovelace");
        assert_eq!(person.biography.as_deref(), Some("A mathematician."));
        assert_eq!(person.birthday.as_deref(), Some("1815-12-10"));
        assert_eq!(person.deathday.as_deref(), Some("1852-11-27"));
        assert_eq!(person.place_of_birth.as_deref(), Some("London"));
        assert_eq!(person.also_known_as, vec!["Augusta Ada King".to_owned()]);
        assert_eq!(count_kind(&jobs, JobKind::Artwork).await, 1);
    }

    #[tokio::test]
    async fn refresh_person_skips_when_provider_has_no_data() {
        let catalog = MockCatalogRepo::new();
        catalog
            .upsert_person(Person {
                id: PersonId("p1".into()),
                name: "Ada".into(),
                external_id: Some("person/1".into()),
                ..Person::default()
            })
            .await
            .unwrap();
        let jobs = MockJobStore::new();
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Full(Box::default()) }),
            jobs.clone(),
        );

        svc.refresh_person(&PersonId("p1".into()), true, None).await.unwrap();

        assert!(bio(&catalog, "p1").await.is_none());
        assert_eq!(count_kind(&jobs, JobKind::Artwork).await, 0);
    }

    #[tokio::test]
    async fn refresh_person_guard_skips_until_forced() {
        let catalog = MockCatalogRepo::new();
        catalog
            .upsert_person(Person {
                id: PersonId("p1".into()),
                name: "Ada".into(),
                external_id: Some("person/1".into()),
                biography: Some("existing".into()),
                ..Person::default()
            })
            .await
            .unwrap();
        let jobs = MockJobStore::new();
        let meta = PersonMetadata { biography: Some("fresh".into()), ..PersonMetadata::default() };
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::PersonInfo(meta) }),
            jobs.clone(),
        );

        svc.refresh_person(&PersonId("p1".into()), false, None).await.unwrap();
        assert_eq!(
            bio(&catalog, "p1").await.as_deref(),
            Some("existing"),
            "unforced refresh must not overwrite an enriched person"
        );

        svc.refresh_person(&PersonId("p1".into()), true, None).await.unwrap();
        assert_eq!(bio(&catalog, "p1").await.as_deref(), Some("fresh"));
    }

    #[tokio::test]
    async fn refresh_person_is_noop_for_missing_or_unlinked() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::PersonInfo(PersonMetadata::default()) }),
            jobs.clone(),
        );

        svc.refresh_person(&PersonId("missing".into()), true, None).await.unwrap();

        catalog
            .upsert_person(Person {
                id: PersonId("p2".into()),
                name: "NoLink".into(),
                ..Person::default()
            })
            .await
            .unwrap();
        svc.refresh_person(&PersonId("p2".into()), true, None).await.unwrap();

        assert_eq!(count_kind(&jobs, JobKind::Artwork).await, 0);
    }

    async fn bio(catalog: &MockCatalogRepo, id: &str) -> Option<String> {
        catalog.get_person(&PersonId(id.into())).await.unwrap().unwrap().biography
    }

    #[tokio::test]
    async fn tv_non_episodic_file_is_skipped() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let svc = enricher(catalog.clone(), None, jobs.clone());

        svc.enrich(&library(LibraryKind::Tv), &report(&["/tv/The Matrix (1999).mkv"]), None).await;

        assert_eq!(catalog.list_series(page()).await.unwrap().total, 0);
        assert_eq!(
            catalog.list_library_versions(&LibraryId("lib".into()), page()).await.unwrap().total,
            0
        );
        assert!(jobs.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn provider_search_error_is_swallowed() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::SearchErr }),
            jobs.clone(),
        );

        svc.enrich(
            &library(LibraryKind::Movie),
            &report(&["/m/The Matrix (1999) 1080p.mkv"]),
            None,
        )
        .await;

        assert_eq!(catalog.list_movies(page()).await.unwrap().total, 1);
        assert_eq!(count_kind(&jobs, JobKind::Artwork).await, 0);
        assert_eq!(count_kind(&jobs, JobKind::Trickplay).await, 1);
    }

    #[tokio::test]
    async fn provider_empty_matches_enqueues_no_artwork() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Empty }),
            jobs.clone(),
        );

        svc.enrich(
            &library(LibraryKind::Movie),
            &report(&["/m/The Matrix (1999) 1080p.mkv"]),
            None,
        )
        .await;

        assert_eq!(catalog.list_movies(page()).await.unwrap().total, 1);
        assert_eq!(count_kind(&jobs, JobKind::Artwork).await, 0);
        assert_eq!(count_kind(&jobs, JobKind::Trickplay).await, 1);
    }

    #[tokio::test]
    async fn provider_fetch_error_is_swallowed() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::FetchErr }),
            jobs.clone(),
        );

        svc.enrich(
            &library(LibraryKind::Movie),
            &report(&["/m/The Matrix (1999) 1080p.mkv"]),
            None,
        )
        .await;

        assert_eq!(count_kind(&jobs, JobKind::Artwork).await, 0);
        assert_eq!(count_kind(&jobs, JobKind::Trickplay).await, 1);
    }

    #[tokio::test]
    async fn artwork_without_poster_or_backdrop_enqueues_no_artwork() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Artwork(vec![art(ArtworkKind::Logo)]) }),
            jobs.clone(),
        );

        svc.enrich(
            &library(LibraryKind::Movie),
            &report(&["/m/The Matrix (1999) 1080p.mkv"]),
            None,
        )
        .await;

        assert_eq!(count_kind(&jobs, JobKind::Artwork).await, 0);
        assert_eq!(count_kind(&jobs, JobKind::Trickplay).await, 1);
    }

    #[tokio::test]
    async fn ingest_error_is_logged_and_swallowed() {
        let catalog = MockCatalogRepo::new();
        catalog.set_fail();
        let jobs = MockJobStore::new();
        let svc = enricher(catalog, None, jobs.clone());

        svc.enrich(
            &library(LibraryKind::Movie),
            &report(&["/m/The Matrix (1999) 1080p.mkv"]),
            None,
        )
        .await;

        assert!(jobs.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn noop_enricher_does_nothing() {
        NoopEnricher.enrich(&library(LibraryKind::Movie), &report(&["/m/x.mkv"]), None).await;
    }

    #[tokio::test]
    async fn scan_persists_unmatched_files() {
        let libraries = MockLibraryRepo::new();
        let svc = enricher_with_libraries(
            MockCatalogRepo::new(),
            None,
            MockJobStore::new(),
            libraries.clone(),
        );

        svc.enrich(&library(LibraryKind::Movie), &report(&["/m/recording.mkv"]), None).await;

        let unmatched = libraries.list_unmatched(&LibraryId("lib".into()), page()).await.unwrap();
        assert_eq!(unmatched.total, 1);
        assert_eq!(unmatched.items[0].path, "/m/recording.mkv");
    }

    #[tokio::test]
    async fn scan_persists_same_resolution_duplicates() {
        let libraries = MockLibraryRepo::new();
        let svc = enricher_with_libraries(
            MockCatalogRepo::new(),
            None,
            MockJobStore::new(),
            libraries.clone(),
        );

        svc.enrich(
            &library(LibraryKind::Movie),
            &report(&["/m/The Matrix (1999) 1080p.mkv", "/m/The Matrix (1999) 2160p.mkv"]),
            None,
        )
        .await;

        let duplicates = libraries.list_duplicates(&LibraryId("lib".into()), page()).await.unwrap();
        assert_eq!(duplicates.total, 1);
        assert_eq!(duplicates.items[0].paths.len(), 2);
    }

    #[tokio::test]
    async fn a_dismissed_duplicate_returns_while_the_files_are_still_there() {
        let libraries = MockLibraryRepo::new();
        let svc = enricher_with_libraries(
            MockCatalogRepo::new(),
            None,
            MockJobStore::new(),
            libraries.clone(),
        );
        let id = LibraryId("lib".into());
        let both = report(&["/m/The Matrix (1999) 1080p.mkv", "/m/The Matrix (1999) 2160p.mkv"]);

        svc.enrich(&library(LibraryKind::Movie), &both, None).await;
        let candidate = libraries.list_duplicates(&id, page()).await.unwrap().items[0].id.clone();
        libraries.set_duplicate_status(&candidate, ResolutionStatus::Dismissed).await.unwrap();
        assert_eq!(libraries.list_duplicates(&id, page()).await.unwrap().total, 0);

        svc.enrich(&library(LibraryKind::Movie), &both, None).await;

        assert_eq!(
            libraries.list_duplicates(&id, page()).await.unwrap().total,
            1,
            "both files are still on disk, so the warning is still true"
        );
    }

    #[tokio::test]
    async fn a_duplicate_settled_on_disk_is_removed_by_the_next_scan() {
        let libraries = MockLibraryRepo::new();
        let svc = enricher_with_libraries(
            MockCatalogRepo::new(),
            None,
            MockJobStore::new(),
            libraries.clone(),
        );
        let id = LibraryId("lib".into());

        svc.enrich(
            &library(LibraryKind::Movie),
            &report(&["/m/The Matrix (1999) 1080p.mkv", "/m/The Matrix (1999) 2160p.mkv"]),
            None,
        )
        .await;
        assert_eq!(libraries.list_duplicates(&id, page()).await.unwrap().total, 1);

        svc.enrich(
            &library(LibraryKind::Movie),
            &report(&["/m/The Matrix (1999) 1080p.mkv"]),
            None,
        )
        .await;

        assert_eq!(
            libraries.list_duplicates(&id, page()).await.unwrap().total,
            0,
            "one file left, so there is nothing left to warn about"
        );
    }

    #[tokio::test]
    async fn an_unmatched_file_that_left_the_disk_leaves_no_tombstone() {
        let libraries = MockLibraryRepo::new();
        let svc = enricher_with_libraries(
            MockCatalogRepo::new(),
            None,
            MockJobStore::new(),
            libraries.clone(),
        );
        let id = LibraryId("lib".into());

        svc.enrich(&library(LibraryKind::Movie), &report(&["/m/recording.mkv"]), None).await;
        let file = libraries.list_unmatched(&id, page()).await.unwrap().items[0].id.clone();
        libraries.set_unmatched_status(&file, ResolutionStatus::Resolved).await.unwrap();

        svc.enrich(
            &library(LibraryKind::Movie),
            &report(&["/m/The Matrix (1999) 1080p.mkv"]),
            None,
        )
        .await;

        assert!(
            libraries.get_unmatched(&file).await.unwrap().is_none(),
            "a resolved row for a file that is gone is pure tombstone"
        );
    }

    #[tokio::test]
    async fn scan_persistence_errors_are_swallowed() {
        let libraries = MockLibraryRepo::new();
        libraries.set_fail_save();
        let svc = enricher_with_libraries(
            MockCatalogRepo::new(),
            None,
            MockJobStore::new(),
            libraries.clone(),
        );

        svc.enrich(
            &library(LibraryKind::Movie),
            &report(&[
                "/m/recording.mkv",
                "/m/The Matrix (1999) 1080p.mkv",
                "/m/The Matrix (1999) 2160p.mkv",
            ]),
            None,
        )
        .await;

        let id = LibraryId("lib".into());
        assert!(libraries.list_unmatched(&id, page()).await.unwrap().items.is_empty());
        assert!(libraries.list_duplicates(&id, page()).await.unwrap().items.is_empty());
    }

    #[tokio::test]
    async fn resolve_existing_attaches_version_under_title() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let svc = enricher(catalog.clone(), None, jobs.clone());

        svc.ingest_resolved(
            &library(LibraryKind::Movie),
            &discovered("/m/Whatever.mkv"),
            &ResolveTarget::Existing(TitleId::Movie(MovieId("m1".into()))),
            None,
        )
        .await
        .unwrap();

        assert_eq!(catalog.list_movies(page()).await.unwrap().total, 0);
        assert_eq!(
            catalog.list_library_versions(&LibraryId("lib".into()), page()).await.unwrap().total,
            1
        );
        assert_eq!(count_kind(&jobs, JobKind::Trickplay).await, 1);
    }

    #[tokio::test]
    async fn resolve_provider_creates_movie_and_version() {
        let catalog = MockCatalogRepo::new();
        let metadata = TitleMetadata {
            title: "The Matrix (Provider)".into(),
            year: Some(1999),
            overview: Some("From provider".into()),
            ..TitleMetadata::default()
        };
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Full(Box::new(metadata)) }),
            MockJobStore::new(),
        );

        svc.ingest_resolved(
            &library(LibraryKind::Movie),
            &discovered("/m/The Matrix (1999) 1080p.mkv"),
            &ResolveTarget::Provider(ExternalId {
                source: "tmdb".into(),
                value: "movie/603".into(),
            }),
            None,
        )
        .await
        .unwrap();

        let movies = catalog.list_movies(page()).await.unwrap();
        assert_eq!(movies.total, 1);
        assert_eq!(movies.items[0].title, "The Matrix (Provider)");
        assert_eq!(movies.items[0].overview.as_deref(), Some("From provider"));
        assert_eq!(
            catalog.list_library_versions(&LibraryId("lib".into()), page()).await.unwrap().total,
            1
        );
    }

    #[tokio::test]
    async fn an_explicit_resolve_overwrites_an_edited_row_and_clears_the_flag() {
        let catalog = MockCatalogRepo::new();
        let file = discovered("/m/The Matrix (1999) 1080p.mkv");
        let target = ResolveTarget::Provider(ExternalId {
            source: "tmdb".into(),
            value: "movie/603".into(),
        });
        let first = enricher(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(TitleMetadata {
                    title: "Wrong Match".into(),
                    ..TitleMetadata::default()
                })),
            }),
            MockJobStore::new(),
        );
        first.ingest_resolved(&library(LibraryKind::Movie), &file, &target, None).await.unwrap();

        let id = catalog.list_movies(page()).await.unwrap().items[0].id.clone();
        let mut edited = catalog.get_movie(&id).await.unwrap().unwrap();
        edited.title = "Hand Written".into();
        edited.manually_edited = true;
        catalog.upsert_movie(edited).await.unwrap();

        let second = enricher(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(TitleMetadata {
                    title: "The Matrix".into(),
                    ..TitleMetadata::default()
                })),
            }),
            MockJobStore::new(),
        );
        second.ingest_resolved(&library(LibraryKind::Movie), &file, &target, None).await.unwrap();

        let relinked = catalog.get_movie(&id).await.unwrap().unwrap();
        assert_eq!(
            relinked.title, "The Matrix",
            "naming a new identity is a deliberate admin action and beats the flag"
        );
        assert!(!relinked.manually_edited);
    }

    #[test]
    fn a_bare_tmdb_id_is_qualified_by_the_library_kind() {
        let bare = ExternalId { source: "tmdb".into(), value: "4629".into() };
        assert_eq!(qualified_provider_id(&bare, LibraryKind::Tv).value, "tv/4629");
        assert_eq!(qualified_provider_id(&bare, LibraryKind::Movie).value, "movie/4629");

        let qualified = ExternalId { source: "tmdb".into(), value: "movie/4629".into() };
        assert_eq!(
            qualified_provider_id(&qualified, LibraryKind::Tv).value,
            "movie/4629",
            "an explicit endpoint is the caller's decision and is left alone"
        );

        for id in [
            ExternalId { source: "imdb".into(), value: "tt0118480".into() },
            ExternalId { source: "tmdb".into(), value: "tt0118480".into() },
        ] {
            assert_eq!(
                qualified_provider_id(&id, LibraryKind::Tv).value,
                "tt0118480",
                "an imdb id is resolved by lookup, not by endpoint"
            );
        }
    }

    #[tokio::test]
    async fn resolve_provider_creates_episode_tree() {
        let catalog = MockCatalogRepo::new();
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Artwork(Vec::new()) }),
            MockJobStore::new(),
        );

        svc.ingest_resolved(
            &library(LibraryKind::Tv),
            &discovered("/tv/Gamma S01E01 720p.mkv"),
            &ResolveTarget::Provider(ExternalId { source: "tmdb".into(), value: "tv/1".into() }),
            None,
        )
        .await
        .unwrap();

        assert_eq!(catalog.list_series(page()).await.unwrap().total, 1);
    }

    #[tokio::test]
    async fn a_scanned_file_carrying_a_provider_tag_resolves_by_id_and_never_searches() {
        let provider = MockMetadataProvider::new();
        let svc = Enricher::new(
            MockCatalogRepo::new(),
            Some(provider.clone()),
            MockJobStore::new(),
            MockLibraryRepo::new(),
        );

        svc.enrich(
            &library(LibraryKind::Movie),
            &report(&["/ext/The Matrix (1999)/The Matrix (1999) [tmdbid-603].mkv"]),
            None,
        )
        .await;

        assert!(provider.searched().is_empty(), "a tagged file must not be guessed at by title");
        assert_eq!(
            provider.fetched(),
            vec![ExternalId { source: "tmdb".into(), value: "movie/603".into() }]
        );
    }

    #[tokio::test]
    async fn a_scanned_file_without_a_tag_still_searches_by_title() {
        let provider = MockMetadataProvider::new();
        let svc = Enricher::new(
            MockCatalogRepo::new(),
            Some(provider.clone()),
            MockJobStore::new(),
            MockLibraryRepo::new(),
        );

        svc.enrich(&library(LibraryKind::Movie), &report(&["/ext/The Matrix (1999).mkv"]), None)
            .await;

        assert_eq!(provider.searched().len(), 1);
        assert_eq!(provider.searched()[0].title, "The Matrix");
        assert!(provider.fetched().is_empty());
    }

    #[tokio::test]
    async fn a_fetched_file_lands_in_a_tv_library_as_a_full_episode_tree() {
        let catalog = MockCatalogRepo::new();
        let metadata = TitleMetadata {
            title: "Gamma".into(),
            external_ids: vec![ExternalId { source: "tmdb".into(), value: "tv/1399".into() }],
            ..TitleMetadata::default()
        };
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::ByIdOnly(Box::new(metadata)) }),
            MockJobStore::new(),
        );

        svc.ingest_fetched(
            &library(LibraryKind::Tv),
            &discovered("/tv/Gamma S01E01 720p.mkv"),
            &ParsedMedia {
                title: "Gamma".into(),
                year: None,
                season: Some(1),
                episode: Some(1),
                quality: Some(Quality::Hd),
                external_id: Some(ExternalId { source: "tmdb".into(), value: "tv/1399".into() }),
            },
            None,
        )
        .await
        .unwrap();

        let series = catalog.list_series(page()).await.unwrap();
        assert_eq!(series.total, 1, "a fetched episode still builds its parent");
        assert_eq!(series.items[0].title, "Gamma");
        let seasons = catalog.list_seasons(&series.items[0].id).await.unwrap();
        assert_eq!(seasons.len(), 1);
        assert_eq!(catalog.list_episodes(&seasons[0].id).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn a_fetched_file_with_no_external_id_still_lands_as_a_bare_title() {
        let catalog = MockCatalogRepo::new();
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::ByIdOnly(Box::default()) }),
            MockJobStore::new(),
        );

        svc.ingest_fetched(
            &library(LibraryKind::Movie),
            &discovered("/m/The Matrix (1999).mkv"),
            &ParsedMedia {
                title: "The Matrix".into(),
                year: Some(1999),
                season: None,
                episode: None,
                quality: None,
                external_id: None,
            },
            None,
        )
        .await
        .unwrap();

        let movies = catalog.list_movies(page()).await.unwrap();
        assert_eq!(movies.total, 1);
        assert_eq!(
            movies.items[0].title, "The Matrix",
            "with nothing to fetch the parsed filename is all there is"
        );
        assert!(movies.items[0].overview.is_none());
    }

    #[tokio::test]
    async fn resolve_provider_without_configured_provider_creates_bare_title() {
        let catalog = MockCatalogRepo::new();
        let svc = enricher(catalog.clone(), None, MockJobStore::new());

        svc.ingest_resolved(
            &library(LibraryKind::Movie),
            &discovered("/m/The Matrix (1999).mkv"),
            &ResolveTarget::Provider(ExternalId {
                source: "tmdb".into(),
                value: "movie/603".into(),
            }),
            None,
        )
        .await
        .unwrap();

        let movies = catalog.list_movies(page()).await.unwrap();
        assert_eq!(movies.total, 1);
        assert!(movies.items[0].overview.is_none());
    }

    #[tokio::test]
    async fn resolve_provider_fetch_error_creates_bare_title() {
        let catalog = MockCatalogRepo::new();
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::FetchErr }),
            MockJobStore::new(),
        );

        svc.ingest_resolved(
            &library(LibraryKind::Movie),
            &discovered("/m/The Matrix (1999).mkv"),
            &ResolveTarget::Provider(ExternalId {
                source: "tmdb".into(),
                value: "movie/603".into(),
            }),
            None,
        )
        .await
        .unwrap();

        assert_eq!(catalog.list_movies(page()).await.unwrap().total, 1);
    }

    #[tokio::test]
    async fn a_provider_relink_that_finds_no_metadata_keeps_the_stored_movie() {
        let catalog = MockCatalogRepo::new();
        let id = movie_id_of(&parse_filename("/m/The Matrix (1999).mkv"));
        catalog.add_movie(Movie {
            id: id.clone(),
            title: "The Matrix".into(),
            sort_title: "matrix".into(),
            year: Some(1999),
            overview: Some("A hacker learns the truth.".into()),
            runtime_minutes: Some(136),
            content_rating: Some(ContentRating { system: "mpaa".into(), code: "r".into() }),
            manually_edited: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::FetchErr }),
            MockJobStore::new(),
        );

        let outcome = svc
            .ingest_resolved(
                &library(LibraryKind::Movie),
                &discovered("/m/The Matrix (1999).mkv"),
                &ResolveTarget::Provider(ExternalId {
                    source: "tmdb".into(),
                    value: "movie/603".into(),
                }),
                None,
            )
            .await
            .unwrap();

        assert_eq!(outcome, ResolveOutcome::Unidentified);
        let stored = catalog.get_movie(&id).await.unwrap().unwrap();
        assert_eq!(stored.title, "The Matrix");
        assert_eq!(stored.overview.as_deref(), Some("A hacker learns the truth."));
        assert_eq!(stored.runtime_minutes, Some(136));
        assert_eq!(
            stored.content_rating.map(|rating| rating.code),
            Some("r".to_owned()),
            "losing the rating would silently stop parental gating on this title"
        );
        assert!(
            stored.manually_edited,
            "a provider that could not be reached must not discard a manual edit"
        );
        assert_eq!(
            catalog.list_library_versions(&LibraryId("lib".into()), page()).await.unwrap().total,
            1,
            "the file half of a relink is filesystem truth and does not need the provider"
        );
    }

    #[tokio::test]
    async fn a_provider_relink_that_finds_no_metadata_keeps_the_stored_episode() {
        let catalog = MockCatalogRepo::new();
        let ids = episode_ids_of(&parse_filename("/tv/Gamma S01E01 720p.mkv")).unwrap();
        catalog.add_series(stored_series(&ids.series.0, "Gamma"));
        catalog.add_season(stored_season(&ids.season.0, &ids.series.0, 1));
        catalog.add_episode(Episode {
            title: "The Beginning".into(),
            overview: Some("Stored overview".into()),
            runtime_minutes: Some(48),
            manually_edited: true,
            ..stored_episode(&ids.episode.0, &ids.season.0, 1)
        });
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::FetchErr }),
            MockJobStore::new(),
        );

        let outcome = svc
            .ingest_resolved(
                &library(LibraryKind::Tv),
                &discovered("/tv/Gamma S01E01 720p.mkv"),
                &ResolveTarget::Provider(ExternalId {
                    source: "tmdb".into(),
                    value: "tv/1".into(),
                }),
                None,
            )
            .await
            .unwrap();

        assert_eq!(outcome, ResolveOutcome::Unidentified);
        let episode = catalog.get_episode(&ids.episode).await.unwrap().unwrap();
        assert_eq!(episode.title, "The Beginning");
        assert_eq!(episode.overview.as_deref(), Some("Stored overview"));
        assert_eq!(episode.runtime_minutes, Some(48));
        assert!(episode.manually_edited);
        let series = catalog.get_series(&ids.series).await.unwrap().unwrap();
        assert_eq!(
            series.title, "Gamma",
            "the parent rows must survive the same way the episode does"
        );
        assert_eq!(
            catalog.get_season(&ids.season).await.unwrap().unwrap().title.as_deref(),
            Some("Stored Season")
        );
    }

    #[tokio::test]
    async fn a_provider_relink_onto_an_unknown_title_still_creates_a_row() {
        let catalog = MockCatalogRepo::new();
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::FetchErr }),
            MockJobStore::new(),
        );

        svc.ingest_resolved(
            &library(LibraryKind::Movie),
            &discovered("/m/The Matrix (1999).mkv"),
            &ResolveTarget::Provider(ExternalId {
                source: "tmdb".into(),
                value: "movie/603".into(),
            }),
            None,
        )
        .await
        .unwrap();

        assert_eq!(
            catalog.list_movies(page()).await.unwrap().total,
            1,
            "skipping the write with no row to protect would leave the version orphaned"
        );
    }

    #[tokio::test]
    async fn resolve_provider_tv_non_episodic_is_skipped() {
        let catalog = MockCatalogRepo::new();
        let svc = enricher(catalog.clone(), None, MockJobStore::new());

        svc.ingest_resolved(
            &library(LibraryKind::Tv),
            &discovered("/tv/The Matrix (1999).mkv"),
            &ResolveTarget::Provider(ExternalId { source: "tmdb".into(), value: "tv/1".into() }),
            None,
        )
        .await
        .unwrap();

        assert_eq!(catalog.list_series(page()).await.unwrap().total, 0);
    }

    fn seed_movie(catalog: &MockCatalogRepo, id: &str, overview: Option<&str>) {
        catalog.add_movie(Movie {
            id: MovieId(id.into()),
            title: "Old Title".into(),
            sort_title: "old title".into(),
            year: Some(1999),
            overview: overview.map(ToOwned::to_owned),
            runtime_minutes: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
    }

    fn refresh_metadata() -> TitleMetadata {
        TitleMetadata {
            overview: Some("Refreshed overview".into()),
            runtime_minutes: Some(120),
            artwork: vec![art(ArtworkKind::Poster)],
            ..TitleMetadata::default()
        }
    }

    fn renaming_metadata(title: &str) -> TitleMetadata {
        TitleMetadata { title: title.into(), ..refresh_metadata() }
    }

    fn german_library() -> MockLibraryRepo {
        let libraries = MockLibraryRepo::new();
        let mut library = library(LibraryKind::Movie);
        library.sort_articles = vec!["der".into(), "die".into(), "das".into()];
        libraries.insert_library(library);
        libraries
    }

    fn seed_version(catalog: &MockCatalogRepo, id: &str, title: TitleId) {
        catalog.add_version(Version {
            id: VersionId(id.into()),
            title,
            library: LibraryId("lib".into()),
            quality: Quality::Fhd,
            container: "mkv".into(),
            path: format!("/m/{id}.mkv"),
            size_bytes: 1,
            duration_ms: 1,
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        });
    }

    #[tokio::test]
    async fn a_renamed_movie_resorts_under_its_library_articles() {
        let catalog = MockCatalogRepo::new();
        seed_movie(&catalog, "m1", None);
        seed_version(&catalog, "v1", TitleId::Movie(MovieId("m1".into())));
        let svc = enricher_with_libraries(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(renaming_metadata("Das Boot"))),
            }),
            MockJobStore::new(),
            german_library(),
        );

        svc.refresh(&TitleRef::Movie(MovieId("m1".into())), None, false, None).await.unwrap();

        let movie = catalog.get_movie(&MovieId("m1".into())).await.unwrap().unwrap();
        assert_eq!(movie.title, "Das Boot");
        assert_eq!(movie.sort_title, "boot, das");
    }

    #[tokio::test]
    async fn a_refresh_rekeys_the_sort_title_even_when_the_name_is_unchanged() {
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(Movie {
            id: MovieId("m1".into()),
            title: "Das Boot".into(),
            sort_title: "das boot".into(),
            year: Some(1981),
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        seed_version(&catalog, "v1", TitleId::Movie(MovieId("m1".into())));
        let svc = enricher_with_libraries(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Full(Box::new(refresh_metadata())) }),
            MockJobStore::new(),
            german_library(),
        );

        svc.refresh(&TitleRef::Movie(MovieId("m1".into())), None, false, None).await.unwrap();

        let movie = catalog.get_movie(&MovieId("m1".into())).await.unwrap().unwrap();
        assert_eq!(movie.title, "Das Boot");
        assert_eq!(movie.sort_title, "boot, das");
    }

    #[tokio::test]
    async fn a_refreshed_series_rekeys_its_sort_title_too() {
        let catalog = MockCatalogRepo::new();
        catalog.add_series(Series {
            id: SeriesId("s1".into()),
            title: "Die Welle".into(),
            sort_title: "die welle".into(),
            year: Some(2008),
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
            title: "Ep".into(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        seed_version(&catalog, "v1", TitleId::Episode(EpisodeId("e1".into())));
        let svc = enricher_with_libraries(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Full(Box::new(refresh_metadata())) }),
            MockJobStore::new(),
            german_library(),
        );

        svc.refresh(&TitleRef::Series(SeriesId("s1".into())), None, false, None).await.unwrap();

        let series = catalog.get_series(&SeriesId("s1".into())).await.unwrap().unwrap();
        assert_eq!(series.title, "Die Welle");
        assert_eq!(series.sort_title, "welle, die");
    }

    #[tokio::test]
    async fn a_renamed_movie_outside_any_library_keeps_its_article() {
        let catalog = MockCatalogRepo::new();
        seed_movie(&catalog, "m1", None);
        let svc = enricher_with_libraries(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(renaming_metadata("Das Boot"))),
            }),
            MockJobStore::new(),
            german_library(),
        );

        svc.refresh(&TitleRef::Movie(MovieId("m1".into())), None, false, None).await.unwrap();

        let movie = catalog.get_movie(&MovieId("m1".into())).await.unwrap().unwrap();
        assert_eq!(movie.sort_title, "das boot");
    }

    #[tokio::test]
    async fn a_renamed_series_resorts_via_an_episode_version() {
        let catalog = MockCatalogRepo::new();
        catalog.add_series(Series {
            id: SeriesId("s1".into()),
            title: "Old Series".into(),
            sort_title: "old series".into(),
            year: Some(2001),
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
            title: "One".into(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        seed_version(&catalog, "v1", TitleId::Episode(EpisodeId("e1".into())));
        let svc = enricher_with_libraries(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(renaming_metadata("Die Welle"))),
            }),
            MockJobStore::new(),
            german_library(),
        );

        svc.refresh(&TitleRef::Series(SeriesId("s1".into())), None, false, None).await.unwrap();

        let series = catalog.get_series(&SeriesId("s1".into())).await.unwrap().unwrap();
        assert_eq!(series.title, "Die Welle");
        assert_eq!(series.sort_title, "welle, die");
    }

    #[tokio::test]
    async fn a_renamed_series_with_no_episode_versions_keeps_its_article() {
        let catalog = MockCatalogRepo::new();
        catalog.add_series(Series {
            id: SeriesId("s1".into()),
            title: "Old Series".into(),
            sort_title: "old series".into(),
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
            title: "One".into(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        let svc = enricher_with_libraries(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(renaming_metadata("Die Welle"))),
            }),
            MockJobStore::new(),
            german_library(),
        );

        svc.refresh(&TitleRef::Series(SeriesId("s1".into())), None, false, None).await.unwrap();

        let series = catalog.get_series(&SeriesId("s1".into())).await.unwrap().unwrap();
        assert_eq!(series.sort_title, "die welle");
    }

    #[tokio::test]
    async fn a_version_in_an_unknown_library_yields_no_articles() {
        let catalog = MockCatalogRepo::new();
        seed_movie(&catalog, "m1", None);
        seed_version(&catalog, "v1", TitleId::Movie(MovieId("m1".into())));
        let svc = enricher_with_libraries(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(renaming_metadata("Das Boot"))),
            }),
            MockJobStore::new(),
            MockLibraryRepo::new(),
        );

        svc.refresh(&TitleRef::Movie(MovieId("m1".into())), None, false, None).await.unwrap();

        let movie = catalog.get_movie(&MovieId("m1".into())).await.unwrap().unwrap();
        assert_eq!(movie.sort_title, "das boot");
    }

    #[tokio::test]
    async fn refresh_movie_applies_metadata_and_enqueues_artwork() {
        let catalog = MockCatalogRepo::new();
        seed_movie(&catalog, "m1", None);
        let jobs = MockJobStore::new();
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Full(Box::new(refresh_metadata())) }),
            jobs.clone(),
        );

        svc.refresh(&TitleRef::Movie(MovieId("m1".into())), None, false, None).await.unwrap();

        let movie = catalog.get_movie(&MovieId("m1".into())).await.unwrap().unwrap();
        assert_eq!(movie.overview.as_deref(), Some("Refreshed overview"));
        assert_eq!(movie.runtime_minutes, Some(120));
        assert_eq!(count_kind(&jobs, JobKind::Artwork).await, 1);
        assert!(
            catalog.list_collections(page()).await.unwrap().items.is_empty(),
            "metadata naming no collection must not invent one"
        );
    }

    #[tokio::test]
    async fn refresh_movie_attaches_the_collection_its_metadata_names() {
        let catalog = MockCatalogRepo::new();
        seed_movie(&catalog, "m1", None);
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(TitleMetadata {
                    collection: Some(tmdb_collection("The Matrix Collection")),
                    ..refresh_metadata()
                })),
            }),
            MockJobStore::new(),
        );
        let movie = TitleRef::Movie(MovieId("m1".into()));

        svc.refresh(&movie, None, false, None).await.unwrap();

        let collection = catalog
            .get_collection(&matrix_collection_id())
            .await
            .unwrap()
            .expect("collection created");
        assert_eq!(collection.name, "The Matrix Collection");
        assert_eq!(collection.movies, vec![MovieId("m1".into())]);

        svc.refresh(&movie, None, false, None).await.unwrap();

        assert_eq!(
            catalog.get_collection(&matrix_collection_id()).await.unwrap().unwrap().movies,
            vec![MovieId("m1".into())],
            "refreshing twice must not add the movie twice"
        );
    }

    #[tokio::test]
    async fn refresh_series_by_external_id_applies_metadata() {
        let catalog = MockCatalogRepo::new();
        catalog.add_series(Series {
            id: SeriesId("s1".into()),
            title: "Old Series".into(),
            sort_title: "old series".into(),
            year: Some(2001),
            overview: Some("Stale".into()),
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Full(Box::new(refresh_metadata())) }),
            MockJobStore::new(),
        );

        svc.refresh(
            &TitleRef::Series(SeriesId("s1".into())),
            Some(&ExternalId { source: "tmdb".into(), value: "tv/1".into() }),
            false,
            None,
        )
        .await
        .unwrap();

        let series = catalog.get_series(&SeriesId("s1".into())).await.unwrap().unwrap();
        assert_eq!(series.overview.as_deref(), Some("Refreshed overview"));
    }

    fn stored_series(id: &str, title: &str) -> Series {
        Series {
            id: SeriesId(id.into()),
            title: title.into(),
            sort_title: title.to_lowercase(),
            year: Some(2001),
            overview: Some("Stale".into()),
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    fn stored_season(id: &str, series: &str, number: u16) -> Season {
        Season {
            id: SeasonId(id.into()),
            series: SeriesId(series.into()),
            number,
            title: Some("Stored Season".into()),
            overview: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    fn stored_episode(id: &str, season: &str, number: u16) -> Episode {
        Episode {
            id: EpisodeId(id.into()),
            season: SeasonId(season.into()),
            number,
            title: format!("Stored {number}"),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    #[tokio::test]
    async fn a_series_refresh_that_finds_no_metadata_changes_nothing() {
        let catalog = MockCatalogRepo::new();
        catalog.add_series(stored_series("s1", "Old Series"));
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Empty }),
            MockJobStore::new(),
        );

        svc.refresh(&TitleRef::Series(SeriesId("s1".into())), None, false, None).await.unwrap();

        let series = catalog.get_series(&SeriesId("s1".into())).await.unwrap().unwrap();
        assert_eq!(series.title, "Old Series");
        assert_eq!(
            series.overview.as_deref(),
            Some("Stale"),
            "a provider that knows nothing must not blank the row"
        );
    }

    #[tokio::test]
    async fn a_season_the_provider_cannot_supply_is_left_alone() {
        let catalog = MockCatalogRepo::new();
        catalog.add_series(stored_series("s1", "Show"));
        catalog.upsert_season(stored_season("s1-1", "s1", 1)).await.unwrap();
        catalog.upsert_episode(stored_episode("s1-1-1", "s1-1", 1)).await.unwrap();

        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Full(Box::new(refresh_metadata())) }),
            MockJobStore::new(),
        );

        svc.refresh(
            &TitleRef::Series(SeriesId("s1".into())),
            Some(&ExternalId { source: "tmdb".into(), value: "tv/1".into() }),
            false,
            None,
        )
        .await
        .unwrap();

        let season = catalog.list_seasons(&SeriesId("s1".into())).await.unwrap()[0].clone();
        assert_eq!(
            season.title.as_deref(),
            Some("Stored Season"),
            "a failed season fetch skips that season instead of wiping it"
        );
        let episode = catalog.list_episodes(&SeasonId("s1-1".into())).await.unwrap()[0].clone();
        assert_eq!(episode.title, "Stored 1");
    }

    #[tokio::test]
    async fn an_episode_the_provider_does_not_list_is_left_alone() {
        let catalog = MockCatalogRepo::new();
        catalog.add_series(stored_series("s1", "Show"));
        catalog.upsert_season(stored_season("s1-1", "s1", 1)).await.unwrap();
        for number in [1, 2] {
            catalog
                .upsert_episode(stored_episode(&format!("s1-1-{number}"), "s1-1", number))
                .await
                .unwrap();
        }

        let metadata = TitleMetadata {
            title: "Show".into(),
            external_ids: vec![ExternalId { source: "tmdb".into(), value: "tv/99".into() }],
            ..TitleMetadata::default()
        };
        let season = SeasonArtwork {
            number: 1,
            name: Some("Named Season".into()),
            episodes: vec![EpisodeArtwork {
                number: 1,
                name: Some("Ep One".into()),
                ..EpisodeArtwork::default()
            }],
            ..SeasonArtwork::default()
        };
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Season(Box::new(metadata), season) }),
            MockJobStore::new(),
        );

        svc.refresh(&TitleRef::Series(SeriesId("s1".into())), None, false, None).await.unwrap();

        let episodes = catalog.list_episodes(&SeasonId("s1-1".into())).await.unwrap();
        let titles: Vec<&str> = episodes.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(
            titles,
            vec!["Ep One", "Stored 2"],
            "an episode the provider never mentions keeps what it had"
        );
    }

    fn stored_movie(id: &str) -> Movie {
        Movie {
            id: MovieId(id.into()),
            title: id.into(),
            sort_title: id.into(),
            year: Some(1999),
            overview: Some("Stale".into()),
            runtime_minutes: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    fn tmdb_series_metadata() -> TitleMetadata {
        TitleMetadata {
            external_ids: vec![ExternalId { source: "tmdb".into(), value: "tv/1".into() }],
            ..refresh_metadata()
        }
    }

    async fn edited_show(catalog: &MockCatalogRepo) {
        catalog.add_series(Series {
            manually_edited: true,
            ..stored_series("s1", "Hand Titled Show")
        });
        catalog.upsert_season(stored_season("s1-1", "s1", 1)).await.unwrap();
        for number in [1u16, 2] {
            catalog
                .upsert_episode(Episode {
                    manually_edited: true,
                    ..stored_episode(&format!("s1-1-{number}"), "s1-1", number)
                })
                .await
                .unwrap();
        }
    }

    async fn flags(catalog: &MockCatalogRepo) -> (bool, Vec<bool>) {
        let series = catalog.get_series(&SeriesId("s1".into())).await.unwrap().unwrap();
        let episodes = catalog.list_episodes(&SeasonId("s1-1".into())).await.unwrap();
        (series.manually_edited, episodes.iter().map(|e| e.manually_edited).collect())
    }

    #[tokio::test]
    async fn a_forced_series_refresh_clears_an_episode_the_provider_never_lists() {
        let catalog = MockCatalogRepo::new();
        edited_show(&catalog).await;
        let season = SeasonArtwork {
            number: 1,
            name: Some("First Season".into()),
            overview: None,
            artwork: Vec::new(),
            episodes: vec![EpisodeArtwork {
                number: 1,
                name: Some("Pilot".into()),
                ..EpisodeArtwork::default()
            }],
        };
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Season(Box::new(tmdb_series_metadata()), season),
            }),
            MockJobStore::new(),
        );

        svc.refresh(&TitleRef::Series(SeriesId("s1".into())), None, true, None).await.unwrap();

        let (series, episodes) = flags(&catalog).await;
        assert!(!series, "the series flag goes on a forced refresh");
        assert_eq!(
            episodes,
            vec![false, false],
            "a locally split episode the provider never lists would otherwise keep its edit forever, \
             and no episode-level refresh route exists to reach it"
        );
    }

    #[tokio::test]
    async fn a_forced_series_refresh_clears_flags_when_the_provider_has_nothing() {
        let catalog = MockCatalogRepo::new();
        edited_show(&catalog).await;
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Empty }),
            MockJobStore::new(),
        );

        svc.refresh(&TitleRef::Series(SeriesId("s1".into())), None, true, None).await.unwrap();

        let (series, episodes) = flags(&catalog).await;
        assert!(!series);
        assert_eq!(
            episodes,
            vec![false, false],
            "discarding edits is a decision about our own rows, not about what the provider returned"
        );
    }

    #[tokio::test]
    async fn a_forced_series_refresh_clears_flags_with_no_external_id() {
        let catalog = MockCatalogRepo::new();
        edited_show(&catalog).await;
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Full(Box::new(refresh_metadata())) }),
            MockJobStore::new(),
        );

        svc.refresh(&TitleRef::Series(SeriesId("s1".into())), None, true, None).await.unwrap();

        let (series, episodes) = flags(&catalog).await;
        assert!(!series);
        assert_eq!(
            episodes,
            vec![false, false],
            "with no tmdb id the season walk never runs, so the flags have to be cleared before it"
        );
    }

    #[tokio::test]
    async fn a_refresh_that_was_not_forced_keeps_every_flag() {
        let catalog = MockCatalogRepo::new();
        edited_show(&catalog).await;
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Full(Box::new(tmdb_series_metadata())) }),
            MockJobStore::new(),
        );

        svc.refresh(&TitleRef::Series(SeriesId("s1".into())), None, false, None).await.unwrap();

        let (series, episodes) = flags(&catalog).await;
        assert!(series, "only a forced refresh discards edits");
        assert_eq!(episodes, vec![true, true]);
    }

    #[tokio::test]
    async fn a_forced_movie_refresh_clears_the_flag_when_the_provider_has_nothing() {
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(Movie {
            manually_edited: true,
            title: "Hand Titled".into(),
            ..stored_movie("m1")
        });
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Empty }),
            MockJobStore::new(),
        );

        svc.refresh(&TitleRef::Movie(MovieId("m1".into())), None, true, None).await.unwrap();

        let movie = catalog.get_movie(&MovieId("m1".into())).await.unwrap().unwrap();
        assert!(
            !movie.manually_edited,
            "the movie path had the same gate, and an unreachable provider must not silently keep the pin"
        );
        assert_eq!(
            movie.title, "Hand Titled",
            "clearing the flag unpins the row, it does not invent provider values"
        );
    }

    #[tokio::test]
    async fn refresh_series_populates_existing_seasons_and_episodes() {
        let catalog = MockCatalogRepo::new();
        catalog.add_series(Series {
            id: SeriesId("s1".into()),
            title: "Show".into(),
            sort_title: "show".into(),
            year: None,
            overview: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog
            .upsert_season(Season {
                id: SeasonId("s1-1".into()),
                series: SeriesId("s1".into()),
                number: 1,
                title: Some("Season 1".into()),
                overview: None,
                added_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
                artwork: Vec::new(),
            })
            .await
            .unwrap();
        catalog
            .upsert_episode(Episode {
                id: EpisodeId("s1-1-1".into()),
                season: SeasonId("s1-1".into()),
                number: 1,
                title: "Episode 1".into(),
                overview: None,
                runtime_minutes: None,
                air_date: None,
                manually_edited: false,
                added_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
                artwork: Vec::new(),
            })
            .await
            .unwrap();

        let metadata = TitleMetadata {
            title: "Show".into(),
            external_ids: vec![ExternalId { source: "tmdb".into(), value: "tv/99".into() }],
            ..TitleMetadata::default()
        };
        let season = SeasonArtwork {
            number: 1,
            name: Some("Named Season".into()),
            overview: Some("Season overview".into()),
            artwork: vec![art(ArtworkKind::Poster)],
            episodes: vec![EpisodeArtwork {
                number: 1,
                name: Some("Ep One".into()),
                overview: Some("Ep overview".into()),
                air_date: Some("2010-10-01".into()),
                runtime_minutes: Some(42),
                artwork: vec![art(ArtworkKind::Backdrop)],
            }],
        };
        let jobs = MockJobStore::new();
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Season(Box::new(metadata), season) }),
            jobs.clone(),
        );

        svc.refresh(&TitleRef::Series(SeriesId("s1".into())), None, false, None).await.unwrap();

        let refreshed_season =
            catalog.list_seasons(&SeriesId("s1".into())).await.unwrap()[0].clone();
        assert_eq!(refreshed_season.title.as_deref(), Some("Named Season"));
        assert_eq!(refreshed_season.overview.as_deref(), Some("Season overview"));
        let refreshed_ep =
            catalog.list_episodes(&SeasonId("s1-1".into())).await.unwrap()[0].clone();
        assert_eq!(refreshed_ep.title, "Ep One");
        assert_eq!(refreshed_ep.overview.as_deref(), Some("Ep overview"));
        assert_eq!(refreshed_ep.runtime_minutes, Some(42));
        assert!(refreshed_ep.air_date.is_some());

        let owners: Vec<ArtworkOwner> = jobs
            .list()
            .await
            .unwrap()
            .iter()
            .filter(|j| j.kind == JobKind::Artwork)
            .map(|j| ArtworkJobPayload::decode(&j.payload).unwrap().owner)
            .collect();
        assert!(owners.iter().any(|o| matches!(o, ArtworkOwner::Season(_))));
        assert!(owners.iter().any(|o| matches!(o, ArtworkOwner::Episode(_))));
    }

    async fn flag_movie(catalog: &MockCatalogRepo, id: &str) {
        let mut movie = catalog.get_movie(&MovieId(id.into())).await.unwrap().unwrap();
        movie.manually_edited = true;
        catalog.upsert_movie(movie).await.unwrap();
    }

    #[tokio::test]
    async fn a_non_force_refresh_leaves_an_edited_movie_row_alone() {
        let catalog = MockCatalogRepo::new();
        seed_movie(&catalog, "m1", Some("Hand written"));
        flag_movie(&catalog, "m1").await;
        let jobs = MockJobStore::new();
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Full(Box::new(refresh_metadata())) }),
            jobs.clone(),
        );

        svc.refresh(&TitleRef::Movie(MovieId("m1".into())), None, false, None).await.unwrap();

        let movie = catalog.get_movie(&MovieId("m1".into())).await.unwrap().unwrap();
        assert_eq!(movie.title, "Old Title");
        assert_eq!(movie.overview.as_deref(), Some("Hand written"));
        assert!(movie.runtime_minutes.is_none());
        assert!(movie.manually_edited);
        assert_eq!(
            count_kind(&jobs, JobKind::Artwork).await,
            1,
            "the row is held but enrichment still runs"
        );
    }

    #[tokio::test]
    async fn a_force_refresh_overwrites_an_edited_movie_and_clears_the_flag() {
        let catalog = MockCatalogRepo::new();
        seed_movie(&catalog, "m1", Some("Hand written"));
        flag_movie(&catalog, "m1").await;
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Full(Box::new(refresh_metadata())) }),
            MockJobStore::new(),
        );

        svc.refresh(&TitleRef::Movie(MovieId("m1".into())), None, true, None).await.unwrap();

        let movie = catalog.get_movie(&MovieId("m1".into())).await.unwrap().unwrap();
        assert_eq!(movie.overview.as_deref(), Some("Refreshed overview"));
        assert_eq!(movie.runtime_minutes, Some(120));
        assert!(!movie.manually_edited);
    }

    async fn store_tmdb_id(catalog: &MockCatalogRepo, title: TitleRef, value: &str) {
        catalog
            .set_title_enrichment(
                &title,
                &TitleEnrichment {
                    external_ids: vec![ExternalId { source: "tmdb".into(), value: value.into() }],
                    ..TitleEnrichment::default()
                },
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn a_force_refresh_re_fetches_the_stored_id_not_the_edited_title() {
        let catalog = MockCatalogRepo::new();
        seed_movie(&catalog, "m1", Some("Hand written"));
        store_tmdb_id(&catalog, TitleRef::Movie(MovieId("m1".into())), "movie/603").await;
        let mut edited = catalog.get_movie(&MovieId("m1".into())).await.unwrap().unwrap();
        edited.title = "Something The Provider Cannot Find".into();
        edited.year = Some(1234);
        edited.manually_edited = true;
        catalog.upsert_movie(edited).await.unwrap();

        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::ByIdOnly(Box::new(refresh_metadata())) }),
            MockJobStore::new(),
        );

        svc.refresh(&TitleRef::Movie(MovieId("m1".into())), None, true, None).await.unwrap();

        let movie = catalog.get_movie(&MovieId("m1".into())).await.unwrap().unwrap();
        assert_eq!(movie.overview.as_deref(), Some("Refreshed overview"));
        assert_eq!(movie.runtime_minutes, Some(120));
        assert!(
            !movie.manually_edited,
            "a force refresh that finds nothing leaves the flag set, which is the reported bug"
        );
    }

    #[tokio::test]
    async fn a_series_force_refresh_also_re_fetches_its_stored_id() {
        let catalog = MockCatalogRepo::new();
        catalog.add_series(Series {
            id: SeriesId("s1".into()),
            title: "Renamed By Hand".into(),
            sort_title: "renamed by hand".into(),
            year: Some(1234),
            overview: Some("Hand written".into()),
            content_rating: None,
            manually_edited: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        store_tmdb_id(&catalog, TitleRef::Series(SeriesId("s1".into())), "tv/1").await;

        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::ByIdOnly(Box::new(refresh_metadata())) }),
            MockJobStore::new(),
        );

        svc.refresh(&TitleRef::Series(SeriesId("s1".into())), None, true, None).await.unwrap();

        let series = catalog.get_series(&SeriesId("s1".into())).await.unwrap().unwrap();
        assert_eq!(series.overview.as_deref(), Some("Refreshed overview"));
        assert!(!series.manually_edited);
    }

    #[tokio::test]
    async fn a_title_with_no_stored_id_still_falls_back_to_searching() {
        let catalog = MockCatalogRepo::new();
        seed_movie(&catalog, "m1", None);
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Full(Box::new(refresh_metadata())) }),
            MockJobStore::new(),
        );

        svc.refresh(&TitleRef::Movie(MovieId("m1".into())), None, false, None).await.unwrap();

        let movie = catalog.get_movie(&MovieId("m1".into())).await.unwrap().unwrap();
        assert_eq!(movie.overview.as_deref(), Some("Refreshed overview"));
    }

    #[tokio::test]
    async fn an_explicit_external_id_still_wins_over_the_stored_one() {
        let catalog = MockCatalogRepo::new();
        seed_movie(&catalog, "m1", None);
        store_tmdb_id(&catalog, TitleRef::Movie(MovieId("m1".into())), "movie/603").await;
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::ByIdOnly(Box::new(refresh_metadata())) }),
            MockJobStore::new(),
        );

        svc.refresh(
            &TitleRef::Movie(MovieId("m1".into())),
            Some(&ExternalId { source: "tmdb".into(), value: "movie/999".into() }),
            false,
            None,
        )
        .await
        .unwrap();

        let movie = catalog.get_movie(&MovieId("m1".into())).await.unwrap().unwrap();
        assert_eq!(movie.overview.as_deref(), Some("Refreshed overview"));
    }

    #[tokio::test]
    async fn a_non_force_refresh_leaves_an_edited_series_row_alone() {
        let catalog = MockCatalogRepo::new();
        catalog.add_series(Series {
            id: SeriesId("s1".into()),
            title: "Old Show".into(),
            sort_title: "old show".into(),
            year: None,
            overview: Some("Hand written".into()),
            content_rating: None,
            manually_edited: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider { mode: ProviderMode::Full(Box::new(refresh_metadata())) }),
            MockJobStore::new(),
        );

        svc.refresh(&TitleRef::Series(SeriesId("s1".into())), None, false, None).await.unwrap();

        let series = catalog.get_series(&SeriesId("s1".into())).await.unwrap().unwrap();
        assert_eq!(series.title, "Old Show");
        assert_eq!(series.overview.as_deref(), Some("Hand written"));
        assert!(series.manually_edited);
    }

    #[tokio::test]
    async fn refresh_without_metadata_keeps_existing_row() {
        let catalog = MockCatalogRepo::new();
        seed_movie(&catalog, "m1", Some("Keep me"));
        let jobs = MockJobStore::new();
        let svc = enricher(catalog.clone(), None, jobs.clone());

        svc.refresh(&TitleRef::Movie(MovieId("m1".into())), None, false, None).await.unwrap();

        let movie = catalog.get_movie(&MovieId("m1".into())).await.unwrap().unwrap();
        assert_eq!(movie.overview.as_deref(), Some("Keep me"));
        assert_eq!(count_kind(&jobs, JobKind::Artwork).await, 0);
    }

    async fn seed_flagged_episode_show(catalog: &MockCatalogRepo, episode_edited: bool) {
        catalog.add_series(Series {
            id: SeriesId("s1".into()),
            title: "Show".into(),
            sort_title: "show".into(),
            year: None,
            overview: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog
            .upsert_season(Season {
                id: SeasonId("s1-1".into()),
                series: SeriesId("s1".into()),
                number: 1,
                title: Some("Season 1".into()),
                overview: None,
                added_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
                artwork: Vec::new(),
            })
            .await
            .unwrap();
        catalog
            .upsert_episode(Episode {
                id: EpisodeId("s1-1-1".into()),
                season: SeasonId("s1-1".into()),
                number: 1,
                title: "Hand Written".into(),
                overview: Some("Hand written".into()),
                runtime_minutes: None,
                air_date: None,
                manually_edited: episode_edited,
                added_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
                artwork: Vec::new(),
            })
            .await
            .unwrap();
    }

    fn season_refresh_provider() -> MockProvider {
        let metadata = TitleMetadata {
            title: "Show".into(),
            external_ids: vec![ExternalId { source: "tmdb".into(), value: "tv/99".into() }],
            ..TitleMetadata::default()
        };
        let season = SeasonArtwork {
            number: 1,
            name: Some("Named Season".into()),
            overview: None,
            artwork: Vec::new(),
            episodes: vec![EpisodeArtwork {
                number: 1,
                name: Some("Ep One".into()),
                overview: Some("Ep overview".into()),
                air_date: Some("2010-10-01".into()),
                runtime_minutes: Some(42),
                artwork: Vec::new(),
            }],
        };
        MockProvider { mode: ProviderMode::Season(Box::new(metadata), season) }
    }

    async fn only_episode(catalog: &MockCatalogRepo) -> Episode {
        catalog.list_episodes(&SeasonId("s1-1".into())).await.unwrap()[0].clone()
    }

    #[tokio::test]
    async fn a_non_force_refresh_leaves_an_edited_episode_row_alone() {
        let catalog = MockCatalogRepo::new();
        seed_flagged_episode_show(&catalog, true).await;
        let svc = enricher(catalog.clone(), Some(season_refresh_provider()), MockJobStore::new());

        svc.refresh(&TitleRef::Series(SeriesId("s1".into())), None, false, None).await.unwrap();

        let episode = only_episode(&catalog).await;
        assert_eq!(episode.title, "Hand Written");
        assert_eq!(episode.overview.as_deref(), Some("Hand written"));
        assert!(episode.runtime_minutes.is_none());
        assert!(episode.manually_edited);

        let season = catalog.list_seasons(&SeriesId("s1".into())).await.unwrap()[0].clone();
        assert_eq!(
            season.title.as_deref(),
            Some("Named Season"),
            "seasons carry no flag and are always rewritten"
        );
    }

    #[tokio::test]
    async fn a_series_force_refresh_clears_every_episode_flag_beneath_it() {
        let catalog = MockCatalogRepo::new();
        seed_flagged_episode_show(&catalog, true).await;
        let svc = enricher(catalog.clone(), Some(season_refresh_provider()), MockJobStore::new());

        svc.refresh(&TitleRef::Series(SeriesId("s1".into())), None, true, None).await.unwrap();

        let episode = only_episode(&catalog).await;
        assert_eq!(episode.title, "Ep One");
        assert_eq!(episode.overview.as_deref(), Some("Ep overview"));
        assert_eq!(episode.runtime_minutes, Some(42));
        assert!(!episode.manually_edited);
    }

    #[tokio::test]
    async fn an_unedited_episode_refreshes_normally() {
        let catalog = MockCatalogRepo::new();
        seed_flagged_episode_show(&catalog, false).await;
        let svc = enricher(catalog.clone(), Some(season_refresh_provider()), MockJobStore::new());

        svc.refresh(&TitleRef::Series(SeriesId("s1".into())), None, false, None).await.unwrap();

        let episode = only_episode(&catalog).await;
        assert_eq!(episode.title, "Ep One");
        assert!(!episode.manually_edited);
    }

    #[tokio::test]
    async fn refresh_missing_titles_are_noops() {
        let catalog = MockCatalogRepo::new();
        let svc = enricher(catalog.clone(), None, MockJobStore::new());

        svc.refresh(&TitleRef::Movie(MovieId("ghost".into())), None, false, None).await.unwrap();
        svc.refresh(&TitleRef::Series(SeriesId("ghost".into())), None, false, None).await.unwrap();

        assert_eq!(catalog.list_movies(page()).await.unwrap().total, 0);
        assert_eq!(catalog.list_series(page()).await.unwrap().total, 0);
    }

    #[test]
    fn container_of_extracts_extension_or_defaults() {
        assert_eq!(container_of("/m/Movie.MKV"), "mkv");
        assert_eq!(container_of("/m/nested.dir/clip.mp4"), "mp4");
        assert_eq!(container_of("/m/no_extension"), "bin");
    }

    #[test]
    fn derive_id_is_deterministic_and_distinct_per_kind() {
        assert_eq!(derive_id("movie", "the matrix:1999"), derive_id("movie", "the matrix:1999"));
        assert_ne!(derive_id("movie", "the matrix:1999"), derive_id("series", "the matrix:1999"));
    }
}

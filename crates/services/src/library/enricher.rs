use std::future::Future;

use domain::catalog::{
    ArtworkId, ArtworkOwner, Collection, CollectionId, Episode, EpisodeId, Movie, MovieId, Season,
    SeasonId, Series, SeriesId, TitleId, TitleRef, Version, VersionId,
};
use domain::common::Quality;
use domain::error::RepositoryError;
use domain::job::{Job, JobId, JobKind, JobPriority, JobStatus};
use domain::library::{
    DiscoveredFile, Library, LibraryKind, MatchedGroup, ParsedMedia, ResolveTarget, ScanReport,
};
use domain::media::{AudioTrack, SubtitleFile, SubtitleFileId, SubtitleSource};
use domain::metadata::{
    Artwork, ArtworkKind, CollectionMeta, Credit, ExternalId, Genre, GenreId, MediaKind,
    MetadataProvider, MetadataQuery, Person, PersonId, Studio, StudioId, TitleEnrichment,
    TitleMetadata,
};
use domain::repository::{CatalogRepository, JobRepository, LibraryRepository};
use jiff::Timestamp;
use tracing::{debug, warn};
use uuid::Uuid;

use super::{
    ArtworkJobItem, ArtworkJobPayload, Matcher, SubtitleJobPayload, TranscriptionJobPayload,
    TrickplayJobPayload, discover_subtitles, find_duplicates, normalize_title, parse_filename,
    select_audio_track, translation_job,
};

const ID_NAMESPACE: Uuid = Uuid::NAMESPACE_URL;

pub trait ScanEnricher {
    fn enrich(
        &self,
        library: &Library,
        report: &ScanReport,
        parent: Option<&JobId>,
    ) -> impl Future<Output = ()> + Send;
}

pub trait ResolveIngester {
    fn ingest_resolved(
        &self,
        library: &Library,
        file: &DiscoveredFile,
        target: &ResolveTarget,
        parent: Option<&JobId>,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
}

pub trait MetadataRefresher {
    fn refresh(
        &self,
        title: &TitleRef,
        external_id: Option<&ExternalId>,
        parent: Option<&JobId>,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
}

pub struct NoopEnricher;

impl ScanEnricher for NoopEnricher {
    async fn enrich(&self, _library: &Library, _report: &ScanReport, _parent: Option<&JobId>) {}
}

pub struct Enricher<C, M, J, L> {
    catalog: C,
    provider: Option<M>,
    jobs: J,
    libraries: L,
    matcher: Matcher,
    subtitle_languages: Vec<String>,
    transcription_enabled: bool,
    translation_languages: Vec<String>,
}

struct SubtitleContext {
    imdb_id: Option<String>,
    title: String,
    season: Option<u16>,
    episode: Option<u16>,
}

impl<C, M, J, L> Enricher<C, M, J, L> {
    pub fn new(catalog: C, provider: Option<M>, jobs: J, libraries: L) -> Self {
        Self {
            catalog,
            provider,
            jobs,
            libraries,
            matcher: Matcher::new(),
            subtitle_languages: Vec::new(),
            transcription_enabled: false,
            translation_languages: Vec::new(),
        }
    }

    pub fn with_subtitle_languages(mut self, languages: Vec<String>) -> Self {
        self.subtitle_languages = languages;
        self
    }

    pub fn with_transcription(mut self, enabled: bool) -> Self {
        self.transcription_enabled = enabled;
        self
    }

    pub fn with_translation_languages(mut self, languages: Vec<String>) -> Self {
        self.translation_languages = languages;
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
            if let Err(err) = self.ingest_group(library, group, parent).await {
                warn!(library = %library.id.0, "scan enrichment failed: {err}");
            }
        }
        for duplicate in find_duplicates(&report.matched) {
            if let Err(err) = self
                .libraries
                .insert_duplicate(&library.id, duplicate)
                .await
            {
                warn!(library = %library.id.0, "persisting duplicate failed: {err}");
            }
        }
        for file in report.unmatched {
            if let Err(err) = self.libraries.insert_unmatched(file).await {
                warn!(library = %library.id.0, "persisting unmatched file failed: {err}");
            }
        }
        if let Err(err) = self
            .catalog
            .reconcile_library_versions(&library.id, &present)
            .await
        {
            warn!(library = %library.id.0, "reconciling library versions failed: {err}");
        }
    }
}

impl<C, M, J, L> Enricher<C, M, J, L>
where
    C: CatalogRepository + Send + Sync,
    M: MetadataProvider + Send + Sync,
    J: JobRepository + Send + Sync,
{
    async fn ingest_group(
        &self,
        library: &Library,
        group: &MatchedGroup,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        match library.kind {
            LibraryKind::Movie => self.ingest_movie(library, group, parent).await,
            LibraryKind::Tv => self.ingest_episode(library, group, parent).await,
        }
    }

    async fn ingest_movie(
        &self,
        library: &Library,
        group: &MatchedGroup,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        let metadata = self
            .fetch_metadata(MediaKind::Movie, &group.parsed.title, group.parsed.year)
            .await;
        self.write_movie(library, &group.parsed, &group.files, metadata, parent)
            .await
    }

    async fn write_movie(
        &self,
        library: &Library,
        parsed: &ParsedMedia,
        files: &[DiscoveredFile],
        metadata: Option<TitleMetadata>,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        let slug = normalize_title(&parsed.title);
        let movie_id = MovieId(derive_id(
            "movie",
            &format!("{slug}:{}", parsed.year.unwrap_or(0)),
        ));
        let now = Timestamp::now();
        self.catalog
            .upsert_movie(Movie {
                id: movie_id.clone(),
                title: parsed.title.clone(),
                year: parsed.year,
                overview: metadata.as_ref().and_then(|m| m.overview.clone()),
                runtime_minutes: metadata.as_ref().and_then(|m| m.runtime_minutes),
                content_rating: metadata.as_ref().and_then(|m| m.content_rating.clone()),
                added_at: now,
                updated_at: now,
                artwork: Vec::new(),
            })
            .await?;
        let subtitle = SubtitleContext {
            imdb_id: imdb_id(metadata.as_ref()),
            title: parsed.title.clone(),
            season: None,
            episode: None,
        };
        for file in files {
            self.ingest_file(
                file,
                TitleId::Movie(movie_id.clone()),
                library,
                parsed.quality,
                &subtitle,
                parent,
            )
            .await?;
        }
        if let Some(metadata) = &metadata {
            self.persist_enrichment(&TitleRef::Movie(movie_id.clone()), metadata)
                .await?;
            if let Some(collection) = &metadata.collection {
                self.attach_to_collection(&movie_id, collection, parent)
                    .await?;
            }
        }
        let artwork = metadata.map(|m| m.artwork).unwrap_or_default();
        self.enqueue_artwork(ArtworkOwner::Movie(movie_id), artwork, parent)
            .await
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
        let collection = match self.catalog.get_collection(&id).await? {
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
        self.catalog.upsert_collection(collection).await?;
        self.enqueue_artwork(ArtworkOwner::Collection(id), meta.artwork.clone(), parent)
            .await
    }

    async fn ingest_episode(
        &self,
        library: &Library,
        group: &MatchedGroup,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        let metadata = self
            .fetch_metadata(MediaKind::Series, &group.parsed.title, group.parsed.year)
            .await;
        self.write_episode(library, &group.parsed, &group.files, metadata, parent)
            .await
    }

    async fn write_episode(
        &self,
        library: &Library,
        parsed: &ParsedMedia,
        files: &[DiscoveredFile],
        metadata: Option<TitleMetadata>,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        let (Some(season_no), Some(episode_no)) = (parsed.season, parsed.episode) else {
            debug!("skipping non-episodic file in tv library");
            return Ok(());
        };
        let series_key = normalize_title(&parsed.title);
        let series_id = SeriesId(derive_id("series", &series_key));
        let season_id = SeasonId(derive_id("season", &format!("{series_key}:{season_no}")));
        let episode_id = EpisodeId(derive_id(
            "episode",
            &format!("{series_key}:{season_no}:{episode_no}"),
        ));
        let now = Timestamp::now();

        self.catalog
            .upsert_series(Series {
                id: series_id.clone(),
                title: parsed.title.clone(),
                year: parsed.year,
                overview: metadata.as_ref().and_then(|m| m.overview.clone()),
                content_rating: metadata.as_ref().and_then(|m| m.content_rating.clone()),
                added_at: now,
                updated_at: now,
                artwork: Vec::new(),
            })
            .await?;
        self.catalog
            .upsert_season(Season {
                id: season_id.clone(),
                series: series_id.clone(),
                number: season_no,
                title: Some(format!("Season {season_no}")),
                overview: None,
                added_at: now,
                updated_at: now,
                artwork: Vec::new(),
            })
            .await?;
        self.catalog
            .upsert_episode(Episode {
                id: episode_id.clone(),
                season: season_id,
                number: episode_no,
                title: format!("Episode {episode_no}"),
                overview: None,
                runtime_minutes: None,
                air_date: None,
                added_at: now,
                updated_at: now,
                artwork: Vec::new(),
            })
            .await?;
        let subtitle = SubtitleContext {
            imdb_id: imdb_id(metadata.as_ref()),
            title: parsed.title.clone(),
            season: Some(season_no),
            episode: Some(episode_no),
        };
        for file in files {
            self.ingest_file(
                file,
                TitleId::Episode(episode_id.clone()),
                library,
                parsed.quality,
                &subtitle,
                parent,
            )
            .await?;
        }
        if let Some(metadata) = &metadata {
            self.persist_enrichment(&TitleRef::Series(series_id.clone()), metadata)
                .await?;
        }
        let artwork = metadata.map(|m| m.artwork).unwrap_or_default();
        self.enqueue_artwork(ArtworkOwner::Series(series_id), artwork, parent)
            .await
    }

    async fn ingest_file(
        &self,
        file: &DiscoveredFile,
        title: TitleId,
        library: &Library,
        quality: Option<Quality>,
        subtitle: &SubtitleContext,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        let version_id = VersionId(derive_id("version", &file.path));
        let now = Timestamp::now();
        self.catalog
            .upsert_version(Version {
                id: version_id.clone(),
                title,
                library: library.id.clone(),
                quality: quality.unwrap_or(Quality::Sd),
                container: container_of(&file.path),
                path: file.path.clone(),
                size_bytes: file.size_bytes,
                duration_ms: file.probe.duration_ms,
                edition: None,
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
                })
                .collect();
        if !subtitle_files.is_empty() {
            self.catalog
                .set_subtitle_files(&version_id, &subtitle_files)
                .await?;
        }
        self.enqueue_trickplay(&version_id, &file.path, file.probe.duration_ms, parent)
            .await?;
        let has_native_subtitle = !subtitle_files.is_empty() || !file.probe.subtitles.is_empty();
        let transcription_wanted =
            self.transcription_enabled && !has_native_subtitle && !file.probe.audio.is_empty();
        let opensubtitles_configured = !self.subtitle_languages.is_empty();
        self.enqueue_subtitles(
            &version_id,
            subtitle,
            transcription_wanted && opensubtitles_configured,
            parent,
        )
        .await?;
        if !opensubtitles_configured {
            self.enqueue_transcription(
                &version_id,
                &file.path,
                has_native_subtitle,
                &file.probe.audio,
                parent,
            )
            .await?;
        }
        self.enqueue_translation(&version_id, has_native_subtitle, parent)
            .await
    }

    async fn enqueue_translation(
        &self,
        version_id: &VersionId,
        has_native_subtitle: bool,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        if !has_native_subtitle {
            return Ok(());
        }
        match translation_job(version_id, &self.translation_languages) {
            Some(mut job) => {
                job.parent_id = parent.cloned();
                self.jobs.enqueue(job).await
            }
            None => Ok(()),
        }
    }

    async fn enqueue_transcription(
        &self,
        version_id: &VersionId,
        source_path: &str,
        has_native_subtitle: bool,
        audio: &[AudioTrack],
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        if !self.transcription_enabled || has_native_subtitle || audio.is_empty() {
            return Ok(());
        }
        let preferred = self.subtitle_languages.first().map(String::as_str);
        let audio_track_index = select_audio_track(audio, preferred);
        let source_language = audio_track_index
            .and_then(|index| audio.iter().find(|track| track.index == index))
            .or_else(|| audio.first())
            .and_then(|track| track.language.as_ref())
            .map(|code| code.0.clone());
        let raw = TranscriptionJobPayload {
            version_id: version_id.clone(),
            source_path: source_path.to_owned(),
            source_language,
            audio_track_index,
        }
        .encode()
        .expect("transcription job payload serializes");
        let now = Timestamp::now();
        self.jobs
            .enqueue(Job {
                id: JobId(Uuid::new_v4().to_string()),
                kind: JobKind::Transcription,
                status: JobStatus::Queued,
                priority: JobPriority::Low,
                payload: raw,
                attempts: 0,
                progress: 0.0,
                available_at: now,
                last_error: None,
                created_at: now,
                updated_at: now,
                started_at: None,
                finished_at: None,
                parent_id: parent.cloned(),
            })
            .await
    }

    async fn enqueue_subtitles(
        &self,
        version_id: &VersionId,
        ctx: &SubtitleContext,
        transcribe_on_miss: bool,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        if self.subtitle_languages.is_empty() {
            return Ok(());
        }
        let raw = SubtitleJobPayload {
            version_id: version_id.clone(),
            imdb_id: ctx.imdb_id.clone(),
            title: Some(ctx.title.clone()),
            languages: self.subtitle_languages.clone(),
            season: ctx.season,
            episode: ctx.episode,
            transcribe_on_miss,
        }
        .encode()
        .expect("subtitle job payload serializes");
        let now = Timestamp::now();
        self.jobs
            .enqueue(Job {
                id: JobId(Uuid::new_v4().to_string()),
                kind: JobKind::Subtitles,
                status: JobStatus::Queued,
                priority: JobPriority::Normal,
                payload: raw,
                attempts: 0,
                progress: 0.0,
                available_at: now,
                last_error: None,
                created_at: now,
                updated_at: now,
                started_at: None,
                finished_at: None,
                parent_id: parent.cloned(),
            })
            .await
    }

    async fn enqueue_trickplay(
        &self,
        version_id: &VersionId,
        source_path: &str,
        duration_ms: u64,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        let raw = TrickplayJobPayload {
            version_id: version_id.clone(),
            source_path: source_path.to_owned(),
            duration_ms,
        }
        .encode()
        .expect("trickplay job payload serializes");
        let now = Timestamp::now();
        self.jobs
            .enqueue(Job {
                id: JobId(Uuid::new_v4().to_string()),
                kind: JobKind::Trickplay,
                status: JobStatus::Queued,
                priority: JobPriority::Normal,
                payload: raw,
                attempts: 0,
                progress: 0.0,
                available_at: now,
                last_error: None,
                created_at: now,
                updated_at: now,
                started_at: None,
                finished_at: None,
                parent_id: parent.cloned(),
            })
            .await
    }

    async fn enqueue_artwork(
        &self,
        owner: ArtworkOwner,
        artwork: Vec<Artwork>,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        let items: Vec<ArtworkJobItem> = artwork
            .into_iter()
            .filter(|art| matches!(art.kind, ArtworkKind::Poster | ArtworkKind::Backdrop))
            .map(|art| ArtworkJobItem {
                id: ArtworkId(Uuid::new_v4().to_string()),
                kind: art.kind,
                url: art.url,
            })
            .collect();
        if items.is_empty() {
            return Ok(());
        }
        let raw = ArtworkJobPayload { owner, items }
            .encode()
            .expect("artwork job payload serializes");
        let now = Timestamp::now();
        self.jobs
            .enqueue(Job {
                id: JobId(Uuid::new_v4().to_string()),
                kind: JobKind::Artwork,
                status: JobStatus::Queued,
                priority: JobPriority::Normal,
                payload: raw,
                attempts: 0,
                progress: 0.0,
                available_at: now,
                last_error: None,
                created_at: now,
                updated_at: now,
                started_at: None,
                finished_at: None,
                parent_id: parent.cloned(),
            })
            .await
    }

    async fn persist_enrichment(
        &self,
        owner: &TitleRef,
        metadata: &TitleMetadata,
    ) -> Result<(), RepositoryError> {
        let mut credits = Vec::new();
        for info in &metadata.cast {
            let person = PersonId(derive_id("person", &info.external_person_id.value));
            self.catalog
                .upsert_person(Person {
                    id: person.clone(),
                    name: info.name.clone(),
                })
                .await?;
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
            .map(|name| Genre {
                id: GenreId(derive_id("genre", name)),
                name: name.clone(),
            })
            .collect();
        let studios = metadata
            .studios
            .iter()
            .map(|name| Studio {
                id: StudioId(derive_id("studio", name)),
                name: name.clone(),
            })
            .collect();
        let enrichment = TitleEnrichment {
            genres,
            credits,
            studios,
            ratings: metadata.ratings.clone(),
            external_ids: metadata.external_ids.clone(),
            extras: Vec::new(),
        };
        self.catalog.set_title_enrichment(owner, &enrichment).await
    }

    async fn fetch_metadata(
        &self,
        kind: MediaKind,
        title: &str,
        year: Option<u16>,
    ) -> Option<TitleMetadata> {
        let provider = self.provider.as_ref()?;
        let query = MetadataQuery {
            title: title.to_owned(),
            year,
            kind,
        };
        let matches = match provider.search(&query).await {
            Ok(matches) => matches,
            Err(err) => {
                debug!("metadata search failed for {title}: {err}");
                return None;
            }
        };
        let first = matches.into_iter().next()?;
        match provider.fetch(&first.external_id).await {
            Ok(metadata) => Some(metadata),
            Err(err) => {
                debug!("metadata fetch failed for {title}: {err}");
                None
            }
        }
    }

    async fn fetch_by_id(&self, id: &ExternalId) -> Option<TitleMetadata> {
        let provider = self.provider.as_ref()?;
        match provider.fetch(id).await {
            Ok(metadata) => Some(metadata),
            Err(err) => {
                debug!("metadata fetch failed for {}: {err}", id.value);
                None
            }
        }
    }

    async fn fetch_refresh(
        &self,
        kind: MediaKind,
        title: &str,
        year: Option<u16>,
        external_id: Option<&ExternalId>,
    ) -> Option<TitleMetadata> {
        match external_id {
            Some(id) => self.fetch_by_id(id).await,
            None => self.fetch_metadata(kind, title, year).await,
        }
    }

    async fn refresh_movie(
        &self,
        id: &MovieId,
        external_id: Option<&ExternalId>,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        let Some(existing) = self.catalog.get_movie(id).await? else {
            debug!("refresh skipped for missing movie {}", id.0);
            return Ok(());
        };
        let Some(metadata) = self
            .fetch_refresh(
                MediaKind::Movie,
                &existing.title,
                existing.year,
                external_id,
            )
            .await
        else {
            return Ok(());
        };
        let mut updated = existing.clone();
        updated.overview = metadata.overview.clone().or(existing.overview);
        updated.runtime_minutes = metadata.runtime_minutes.or(existing.runtime_minutes);
        updated.content_rating = metadata.content_rating.clone().or(existing.content_rating);
        self.catalog.upsert_movie(updated).await?;
        self.persist_enrichment(&TitleRef::Movie(id.clone()), &metadata)
            .await?;
        self.enqueue_artwork(ArtworkOwner::Movie(id.clone()), metadata.artwork, parent)
            .await
    }

    async fn refresh_series(
        &self,
        id: &SeriesId,
        external_id: Option<&ExternalId>,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        let Some(existing) = self.catalog.get_series(id).await? else {
            debug!("refresh skipped for missing series {}", id.0);
            return Ok(());
        };
        let Some(metadata) = self
            .fetch_refresh(
                MediaKind::Series,
                &existing.title,
                existing.year,
                external_id,
            )
            .await
        else {
            return Ok(());
        };
        let mut updated = existing.clone();
        updated.overview = metadata.overview.clone().or(existing.overview);
        updated.content_rating = metadata.content_rating.clone().or(existing.content_rating);
        self.catalog.upsert_series(updated).await?;
        self.persist_enrichment(&TitleRef::Series(id.clone()), &metadata)
            .await?;
        self.enqueue_artwork(ArtworkOwner::Series(id.clone()), metadata.artwork, parent)
            .await
    }
}

impl<C, M, J, L> ResolveIngester for Enricher<C, M, J, L>
where
    C: CatalogRepository + Send + Sync,
    M: MetadataProvider + Send + Sync,
    J: JobRepository + Send + Sync,
    L: Send + Sync,
{
    async fn ingest_resolved(
        &self,
        library: &Library,
        file: &DiscoveredFile,
        target: &ResolveTarget,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        let parsed = parse_filename(&file.path);
        match target {
            ResolveTarget::Existing(title) => {
                let subtitle = SubtitleContext {
                    imdb_id: None,
                    title: parsed.title.clone(),
                    season: parsed.season,
                    episode: parsed.episode,
                };
                self.ingest_file(
                    file,
                    title.clone(),
                    library,
                    parsed.quality,
                    &subtitle,
                    parent,
                )
                .await
            }
            ResolveTarget::Provider(external_id) => {
                let metadata = self.fetch_by_id(external_id).await;
                let files = std::slice::from_ref(file);
                match library.kind {
                    LibraryKind::Movie => {
                        self.write_movie(library, &parsed, files, metadata, parent)
                            .await
                    }
                    LibraryKind::Tv => {
                        self.write_episode(library, &parsed, files, metadata, parent)
                            .await
                    }
                }
            }
        }
    }
}

impl<C, M, J, L> MetadataRefresher for Enricher<C, M, J, L>
where
    C: CatalogRepository + Send + Sync,
    M: MetadataProvider + Send + Sync,
    J: JobRepository + Send + Sync,
    L: Send + Sync,
{
    async fn refresh(
        &self,
        title: &TitleRef,
        external_id: Option<&ExternalId>,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        match title {
            TitleRef::Movie(id) => self.refresh_movie(id, external_id, parent).await,
            TitleRef::Series(id) => self.refresh_series(id, external_id, parent).await,
        }
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

fn imdb_id(metadata: Option<&TitleMetadata>) -> Option<String> {
    metadata?
        .external_ids
        .iter()
        .find(|id| id.source == "imdb")
        .map(|id| id.value.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{MockCatalogRepo, MockJobStore, MockLibraryRepo};
    use domain::error::MetadataError;
    use domain::library::{DiscoveredFile, LibraryId, WatcherStrategy};
    use domain::media::ProbeResult;
    use domain::metadata::{
        ContentRating, CreditInfo, CreditRole, ExternalId, MetadataMatch, Rating, TitleMetadata,
    };

    enum ProviderMode {
        SearchErr,
        Empty,
        FetchErr,
        Artwork(Vec<Artwork>),
        Full(Box<TitleMetadata>),
    }

    struct MockProvider {
        mode: ProviderMode,
    }

    impl MetadataProvider for MockProvider {
        async fn search(
            &self,
            _query: &MetadataQuery,
        ) -> Result<Vec<MetadataMatch>, MetadataError> {
            match self.mode {
                ProviderMode::SearchErr => Err(MetadataError::Backend("boom".into())),
                ProviderMode::Empty => Ok(Vec::new()),
                _ => Ok(vec![MetadataMatch {
                    external_id: ExternalId {
                        source: "tmdb".into(),
                        value: "movie/1".into(),
                    },
                    title: "match".into(),
                    year: None,
                    kind: MediaKind::Movie,
                }]),
            }
        }

        async fn fetch(&self, _id: &ExternalId) -> Result<TitleMetadata, MetadataError> {
            match &self.mode {
                ProviderMode::Artwork(artwork) => Ok(TitleMetadata {
                    artwork: artwork.clone(),
                    ..TitleMetadata::default()
                }),
                ProviderMode::Full(metadata) => Ok((**metadata).clone()),
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
            kind,
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
        domain::common::PageRequest {
            offset: 0,
            limit: 10,
        }
    }

    async fn count_kind(jobs: &MockJobStore, kind: JobKind) -> usize {
        jobs.list()
            .await
            .unwrap()
            .iter()
            .filter(|j| j.kind == kind)
            .count()
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
        let detail = catalog
            .movie_detail(&movies.items[0].id)
            .await
            .unwrap()
            .unwrap();
        assert!(detail.genres.is_empty());
        assert!(detail.credits.is_empty());
        assert!(detail.studios.is_empty());
        assert_eq!(
            catalog
                .list_library_versions(&LibraryId("lib".into()), page())
                .await
                .unwrap()
                .total,
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
        let subtitles = jobs
            .list()
            .await
            .unwrap()
            .into_iter()
            .find(|j| j.kind == JobKind::Subtitles)
            .unwrap();
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
            &audio_scan(
                "/m/The Matrix (1999) 1080p.mkv",
                Vec::new(),
                vec![embedded_subtitle()],
            ),
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
                ExternalId {
                    source: "tmdb".into(),
                    value: "movie/603".into(),
                },
                ExternalId {
                    source: "imdb".into(),
                    value: "tt0133093".into(),
                },
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
            content_rating: Some(ContentRating {
                system: "MPAA".into(),
                code: "R".into(),
            }),
            ratings: vec![Rating {
                source: "tmdb".into(),
                value: 8.5,
            }],
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
            external_ids: vec![ExternalId {
                source: "tmdb".into(),
                value: "movie/603".into(),
            }],
            collection: None,
        };
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(metadata)),
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

        let person = catalog
            .get_person(&PersonId(derive_id("person", "person/1")))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(person.name, "Keanu Reeves");

        assert_eq!(count_kind(&jobs, JobKind::Artwork).await, 1);
        assert_eq!(count_kind(&jobs, JobKind::Trickplay).await, 1);
    }

    fn tmdb_collection(name: &str) -> CollectionMeta {
        CollectionMeta {
            external_id: ExternalId {
                source: "tmdb".into(),
                value: "collection/2344".into(),
            },
            name: name.into(),
            artwork: vec![art(ArtworkKind::Poster), art(ArtworkKind::Backdrop)],
        }
    }

    fn movie_with_collection(name: &str) -> TitleMetadata {
        TitleMetadata {
            collection: Some(tmdb_collection(name)),
            ..TitleMetadata::default()
        }
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
        let artwork = jobs
            .list()
            .await
            .unwrap()
            .into_iter()
            .find(|j| j.kind == JobKind::Artwork)
            .unwrap();
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
            &report(&[
                "/m/The Matrix (1999) 1080p.mkv",
                "/m/The Matrix Reloaded (2003) 1080p.mkv",
            ]),
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

        svc.enrich(&library(LibraryKind::Movie), &report, None)
            .await;
        svc.enrich(&library(LibraryKind::Movie), &report, None)
            .await;

        let collection = catalog
            .get_collection(&matrix_collection_id())
            .await
            .unwrap()
            .expect("collection created");
        assert_eq!(collection.movies.len(), 1);
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

        let collection = catalog
            .get_collection(&matrix_collection_id())
            .await
            .unwrap()
            .unwrap();
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
        let scan = ScanReport {
            discovered: vec![file],
            skipped: Vec::new(),
            total_candidates: 1,
        };

        svc.enrich(&library(LibraryKind::Movie), &scan, None).await;

        let version_id = VersionId(derive_id("version", path));
        let detail = catalog.version_detail(&version_id).await.unwrap().unwrap();
        assert_eq!(detail.audio.len(), 1);
        assert_eq!(detail.audio[0].channels, 6);

        let enqueued = jobs.list().await.unwrap();
        let trickplay = enqueued
            .iter()
            .find(|j| j.kind == JobKind::Trickplay)
            .unwrap();
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
        let scan = ScanReport {
            discovered: vec![file],
            skipped: Vec::new(),
            total_candidates: 1,
        };

        svc.enrich(&library(LibraryKind::Movie), &scan, None).await;

        let version_id = VersionId(derive_id("version", path));
        let detail = catalog.version_detail(&version_id).await.unwrap().unwrap();
        assert_eq!(detail.subtitle_files.len(), 2);
        assert!(
            detail
                .subtitle_files
                .iter()
                .all(|s| matches!(s.source, SubtitleSource::External))
        );
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
            catalog
                .list_library_versions(&LibraryId("lib".into()), page())
                .await
                .unwrap()
                .total,
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
            &report(&[
                "/m/The Matrix (1999) 1080p.mkv",
                "/m/Alien (1979) 1080p.mkv",
            ]),
            None,
        )
        .await;
        let seeded = catalog
            .list_library_versions(&LibraryId("lib".into()), page())
            .await
            .unwrap();
        assert_eq!(seeded.total, 2);
        assert!(seeded.items.iter().all(|v| v.available));

        svc.enrich(&lib, &report(&["/m/The Matrix (1999) 1080p.mkv"]), None)
            .await;

        let after = catalog
            .list_library_versions(&LibraryId("lib".into()), page())
            .await
            .unwrap();
        assert_eq!(after.total, 2);
        assert_eq!(catalog.list_movies(page()).await.unwrap().total, 2);
        let matrix = after
            .items
            .iter()
            .find(|v| v.path == "/m/The Matrix (1999) 1080p.mkv")
            .unwrap();
        let alien = after
            .items
            .iter()
            .find(|v| v.path == "/m/Alien (1979) 1080p.mkv")
            .unwrap();
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

        svc.enrich(
            &library(LibraryKind::Tv),
            &report(&["/tv/Gamma S01E01 720p.mkv"]),
            None,
        )
        .await;

        let series = catalog.list_series(page()).await.unwrap();
        assert_eq!(series.total, 1);
        assert_eq!(series.items[0].title, "Gamma");
        let series_id = series.items[0].id.clone();
        assert_eq!(catalog.list_seasons(&series_id).await.unwrap().len(), 1);
        let season_id = catalog.list_seasons(&series_id).await.unwrap()[0]
            .id
            .clone();
        assert_eq!(catalog.list_episodes(&season_id).await.unwrap().len(), 1);
        assert_eq!(
            catalog
                .list_library_versions(&LibraryId("lib".into()), page())
                .await
                .unwrap()
                .total,
            1
        );

        assert_eq!(count_kind(&jobs, JobKind::Trickplay).await, 1);
        let enqueued = jobs.list().await.unwrap();
        let artwork = enqueued
            .iter()
            .find(|j| j.kind == JobKind::Artwork)
            .unwrap();
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
    async fn tv_non_episodic_file_is_skipped() {
        let catalog = MockCatalogRepo::new();
        let jobs = MockJobStore::new();
        let svc = enricher(catalog.clone(), None, jobs.clone());

        svc.enrich(
            &library(LibraryKind::Tv),
            &report(&["/tv/The Matrix (1999).mkv"]),
            None,
        )
        .await;

        assert_eq!(catalog.list_series(page()).await.unwrap().total, 0);
        assert_eq!(
            catalog
                .list_library_versions(&LibraryId("lib".into()), page())
                .await
                .unwrap()
                .total,
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
            Some(MockProvider {
                mode: ProviderMode::SearchErr,
            }),
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
            Some(MockProvider {
                mode: ProviderMode::Empty,
            }),
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
            Some(MockProvider {
                mode: ProviderMode::FetchErr,
            }),
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
            Some(MockProvider {
                mode: ProviderMode::Artwork(vec![art(ArtworkKind::Logo)]),
            }),
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
        NoopEnricher
            .enrich(&library(LibraryKind::Movie), &report(&["/m/x.mkv"]), None)
            .await;
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

        svc.enrich(
            &library(LibraryKind::Movie),
            &report(&["/m/recording.mkv"]),
            None,
        )
        .await;

        let unmatched = libraries
            .list_unmatched(&LibraryId("lib".into()), page())
            .await
            .unwrap();
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
            &report(&[
                "/m/The Matrix (1999) 1080p.mkv",
                "/m/The Matrix (1999) 2160p.mkv",
            ]),
            None,
        )
        .await;

        let duplicates = libraries
            .list_duplicates(&LibraryId("lib".into()), page())
            .await
            .unwrap();
        assert_eq!(duplicates.total, 1);
        assert_eq!(duplicates.items[0].paths.len(), 2);
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
        assert!(
            libraries
                .list_unmatched(&id, page())
                .await
                .unwrap()
                .items
                .is_empty()
        );
        assert!(
            libraries
                .list_duplicates(&id, page())
                .await
                .unwrap()
                .items
                .is_empty()
        );
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
            catalog
                .list_library_versions(&LibraryId("lib".into()), page())
                .await
                .unwrap()
                .total,
            1
        );
        assert_eq!(count_kind(&jobs, JobKind::Trickplay).await, 1);
    }

    #[tokio::test]
    async fn resolve_provider_creates_movie_and_version() {
        let catalog = MockCatalogRepo::new();
        let metadata = TitleMetadata {
            title: "Ignored".into(),
            year: Some(1999),
            overview: Some("From provider".into()),
            ..TitleMetadata::default()
        };
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(metadata)),
            }),
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
        assert_eq!(movies.items[0].title, "The Matrix");
        assert_eq!(movies.items[0].overview.as_deref(), Some("From provider"));
        assert_eq!(
            catalog
                .list_library_versions(&LibraryId("lib".into()), page())
                .await
                .unwrap()
                .total,
            1
        );
    }

    #[tokio::test]
    async fn resolve_provider_creates_episode_tree() {
        let catalog = MockCatalogRepo::new();
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Artwork(Vec::new()),
            }),
            MockJobStore::new(),
        );

        svc.ingest_resolved(
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

        assert_eq!(catalog.list_series(page()).await.unwrap().total, 1);
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
            Some(MockProvider {
                mode: ProviderMode::FetchErr,
            }),
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
    async fn resolve_provider_tv_non_episodic_is_skipped() {
        let catalog = MockCatalogRepo::new();
        let svc = enricher(catalog.clone(), None, MockJobStore::new());

        svc.ingest_resolved(
            &library(LibraryKind::Tv),
            &discovered("/tv/The Matrix (1999).mkv"),
            &ResolveTarget::Provider(ExternalId {
                source: "tmdb".into(),
                value: "tv/1".into(),
            }),
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
            year: Some(1999),
            overview: overview.map(ToOwned::to_owned),
            runtime_minutes: None,
            content_rating: None,
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

    #[tokio::test]
    async fn refresh_movie_applies_metadata_and_enqueues_artwork() {
        let catalog = MockCatalogRepo::new();
        seed_movie(&catalog, "m1", None);
        let jobs = MockJobStore::new();
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(refresh_metadata())),
            }),
            jobs.clone(),
        );

        svc.refresh(&TitleRef::Movie(MovieId("m1".into())), None, None)
            .await
            .unwrap();

        let movie = catalog
            .get_movie(&MovieId("m1".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(movie.overview.as_deref(), Some("Refreshed overview"));
        assert_eq!(movie.runtime_minutes, Some(120));
        assert_eq!(count_kind(&jobs, JobKind::Artwork).await, 1);
    }

    #[tokio::test]
    async fn refresh_series_by_external_id_applies_metadata() {
        let catalog = MockCatalogRepo::new();
        catalog.add_series(Series {
            id: SeriesId("s1".into()),
            title: "Old Series".into(),
            year: Some(2001),
            overview: Some("Stale".into()),
            content_rating: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        let svc = enricher(
            catalog.clone(),
            Some(MockProvider {
                mode: ProviderMode::Full(Box::new(refresh_metadata())),
            }),
            MockJobStore::new(),
        );

        svc.refresh(
            &TitleRef::Series(SeriesId("s1".into())),
            Some(&ExternalId {
                source: "tmdb".into(),
                value: "tv/1".into(),
            }),
            None,
        )
        .await
        .unwrap();

        let series = catalog
            .get_series(&SeriesId("s1".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(series.overview.as_deref(), Some("Refreshed overview"));
    }

    #[tokio::test]
    async fn refresh_without_metadata_keeps_existing_row() {
        let catalog = MockCatalogRepo::new();
        seed_movie(&catalog, "m1", Some("Keep me"));
        let jobs = MockJobStore::new();
        let svc = enricher(catalog.clone(), None, jobs.clone());

        svc.refresh(&TitleRef::Movie(MovieId("m1".into())), None, None)
            .await
            .unwrap();

        let movie = catalog
            .get_movie(&MovieId("m1".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(movie.overview.as_deref(), Some("Keep me"));
        assert_eq!(count_kind(&jobs, JobKind::Artwork).await, 0);
    }

    #[tokio::test]
    async fn refresh_missing_titles_are_noops() {
        let catalog = MockCatalogRepo::new();
        let svc = enricher(catalog.clone(), None, MockJobStore::new());

        svc.refresh(&TitleRef::Movie(MovieId("ghost".into())), None, None)
            .await
            .unwrap();
        svc.refresh(&TitleRef::Series(SeriesId("ghost".into())), None, None)
            .await
            .unwrap();

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
        assert_eq!(
            derive_id("movie", "the matrix:1999"),
            derive_id("movie", "the matrix:1999")
        );
        assert_ne!(
            derive_id("movie", "the matrix:1999"),
            derive_id("series", "the matrix:1999")
        );
    }
}

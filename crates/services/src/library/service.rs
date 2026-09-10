use std::collections::HashSet;
use std::sync::Arc;

use domain::catalog::{
    Episode, EpisodeEdit, EpisodeId, Movie, MovieEdit, MovieId, SeasonId, Series, SeriesEdit,
    SeriesId, TitleId, TitleListFilter, TitleRef, Version, VersionId,
};
use domain::common::{Page, PageRequest};
use domain::error::LibraryError;
use domain::job::{JobKind, JobPriority};
use domain::library::{
    DuplicateCandidate, DuplicateCandidateId, FetchInput, Library, LibraryId, LibraryKind,
    LibraryOrigin, LibraryUpdate, NewLibrary, ResolutionStatus, ResolveCandidate, ResolveTarget,
    ScanState, ScanStatus, UnmatchedFile, UnmatchedFileId,
};
use domain::media::{CookieInspector, CookieVerdict, SubtitleFileId};
use domain::metadata::{ExternalId, MediaKind, MetadataProvider, MetadataQuery, PersonId};
use domain::repository::{CatalogRepository, JobRepository, LibraryRepository, UserRepository};
use domain::service::LibraryService;
use domain::text::sort_title;
use domain::user::{Principal, UserId};
use jiff::Timestamp;
use url::Url;
use uuid::Uuid;

use super::{
    FetchJobPayload, IngestJobPayload, MetadataJobPayload, RelinkJobPayload, combine_job,
    parse_filename, queue_scan, transcription_job, translation_job_with_source, upscale_job,
};
use crate::acl;
use crate::job::queued_job;

#[derive(Clone)]
pub struct LibraryServiceImpl<L, U, J, C, M> {
    libraries: L,
    users: U,
    jobs: J,
    catalog: C,
    provider: Option<M>,
    transcription_enabled: bool,
    translation_enabled: bool,
    upscaling_enabled: bool,
    content_fetch_enabled: bool,
    cookie_inspector: Option<Arc<dyn CookieInspector>>,
}

impl<L, U, J, C, M> LibraryServiceImpl<L, U, J, C, M> {
    pub fn new(libraries: L, users: U, jobs: J, catalog: C, provider: Option<M>) -> Self {
        Self {
            libraries,
            users,
            jobs,
            catalog,
            provider,
            transcription_enabled: false,
            translation_enabled: false,
            upscaling_enabled: false,
            content_fetch_enabled: false,
            cookie_inspector: None,
        }
    }

    pub fn with_enrichment_flags(
        mut self,
        transcription_enabled: bool,
        translation_enabled: bool,
        upscaling_enabled: bool,
    ) -> Self {
        self.transcription_enabled = transcription_enabled;
        self.translation_enabled = translation_enabled;
        self.upscaling_enabled = upscaling_enabled;
        self
    }

    pub fn with_content_fetch(mut self, content_fetch_enabled: bool) -> Self {
        self.content_fetch_enabled = content_fetch_enabled;
        self
    }

    pub fn with_cookie_inspector(
        mut self,
        cookie_inspector: Option<Arc<dyn CookieInspector>>,
    ) -> Self {
        self.cookie_inspector = cookie_inspector;
        self
    }
}

impl<L, U, J, C, M> LibraryServiceImpl<L, U, J, C, M>
where
    L: LibraryRepository + Sync,
    U: UserRepository + Sync,
    J: JobRepository + Sync,
    C: CatalogRepository + Sync,
    M: MetadataProvider + Sync,
{
    async fn grant_access(&self, user: &UserId, library: &LibraryId) -> Result<(), LibraryError> {
        if self.users.get(user).await?.is_none() {
            return Ok(());
        }
        let mut access: Vec<LibraryId> = self
            .users
            .list_library_access(user)
            .await?
            .into_iter()
            .map(|entry| entry.library)
            .collect();
        access.push(library.clone());
        self.users.set_library_access(user, &access).await?;
        Ok(())
    }

    async fn visible_to(&self, caller: &Principal, id: &LibraryId) -> Result<bool, LibraryError> {
        if acl::is_admin(caller) || acl::is_automation(caller) {
            return Ok(true);
        }
        let access: Vec<LibraryId> = self
            .users
            .list_library_access(&caller.user)
            .await?
            .into_iter()
            .map(|entry| entry.library)
            .collect();
        Ok(acl::can_access_library(&access, id))
    }

    async fn require_library(
        &self,
        caller: &Principal,
        id: &LibraryId,
    ) -> Result<Library, LibraryError> {
        let library = self
            .libraries
            .get(id)
            .await?
            .ok_or(LibraryError::NotFound)?;
        if self.visible_to(caller, id).await? {
            Ok(library)
        } else {
            Err(LibraryError::NotFound)
        }
    }

    async fn title_display(
        &self,
        title: &TitleId,
    ) -> Result<Option<(String, Option<u16>, MediaKind)>, LibraryError> {
        Ok(match title {
            TitleId::Movie(id) => self
                .catalog
                .get_movie(id)
                .await?
                .map(|m| (m.title, m.year, MediaKind::Movie)),
            TitleId::Episode(id) => self
                .catalog
                .get_episode(id)
                .await?
                .map(|e| (e.title, None, MediaKind::Series)),
        })
    }

    async fn series_versions(&self, series: &SeriesId) -> Result<Vec<Version>, LibraryError> {
        Ok(self.catalog.series_versions(series).await?)
    }

    async fn enqueue_relink(
        &self,
        version: &Version,
        target: ResolveTarget,
    ) -> Result<(), LibraryError> {
        let payload = RelinkJobPayload {
            version: version.id.clone(),
            library: version.library.clone(),
            path: version.path.clone(),
            target,
        }
        .encode();
        let now = Timestamp::now();
        self.jobs
            .enqueue(queued_job(
                JobKind::Relink,
                JobPriority::Normal,
                payload,
                None,
                now,
            ))
            .await?;
        Ok(())
    }

    async fn enqueue_metadata(&self, payload: MetadataJobPayload) -> Result<(), LibraryError> {
        let now = Timestamp::now();
        self.jobs
            .enqueue(queued_job(
                JobKind::Metadata,
                JobPriority::Normal,
                payload.encode(),
                None,
                now,
            ))
            .await?;
        Ok(())
    }
}

impl<L, U, J, C, M> LibraryService for LibraryServiceImpl<L, U, J, C, M>
where
    L: LibraryRepository + Sync,
    U: UserRepository + Sync,
    J: JobRepository + Sync,
    C: CatalogRepository + Sync,
    M: MetadataProvider + Sync,
{
    async fn libraries(&self, caller: &Principal) -> Result<Vec<Library>, LibraryError> {
        let all = self.libraries.list().await?;
        if acl::is_admin(caller) {
            return Ok(all);
        }
        let access: Vec<LibraryId> = self
            .users
            .list_library_access(&caller.user)
            .await?
            .into_iter()
            .map(|entry| entry.library)
            .collect();
        Ok(all
            .into_iter()
            .filter(|library| acl::can_access_library(&access, &library.id))
            .collect())
    }

    async fn library(&self, caller: &Principal, id: &LibraryId) -> Result<Library, LibraryError> {
        self.require_library(caller, id).await
    }

    async fn create_library(
        &self,
        caller: &Principal,
        input: NewLibrary,
    ) -> Result<Library, LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        let now = Timestamp::now();
        let library = Library {
            id: LibraryId(Uuid::new_v4().to_string()),
            name: input.name,
            kind: input.kind,
            origin: input.origin,
            roots: input.roots,
            watcher: input.watcher,
            scan_schedule: input.scan_schedule,
            metadata_sources: input.metadata_sources,
            sort_articles: input.sort_articles,
            created_at: now,
            updated_at: now,
        };
        self.libraries.upsert(library.clone()).await?;
        self.grant_access(&caller.user, &library.id).await?;
        Ok(library)
    }

    async fn update_library(
        &self,
        caller: &Principal,
        id: &LibraryId,
        update: LibraryUpdate,
    ) -> Result<Library, LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        let existing = self
            .libraries
            .get(id)
            .await?
            .ok_or(LibraryError::NotFound)?;
        let now = Timestamp::now();
        let library = Library {
            id: id.clone(),
            name: update.name,
            kind: update.kind,
            origin: existing.origin,
            roots: update.roots,
            watcher: update.watcher,
            scan_schedule: update.scan_schedule,
            metadata_sources: update.metadata_sources,
            sort_articles: update.sort_articles,
            created_at: now,
            updated_at: now,
        };
        self.libraries.upsert(library.clone()).await?;
        Ok(library)
    }

    async fn delete_library(&self, caller: &Principal, id: &LibraryId) -> Result<(), LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        self.libraries
            .get(id)
            .await?
            .ok_or(LibraryError::NotFound)?;
        self.catalog.reconcile_library_versions(id, &[]).await?;
        self.users.revoke_library_access(id).await?;
        self.libraries.delete(id).await?;
        Ok(())
    }

    async fn scan_state(
        &self,
        caller: &Principal,
        id: &LibraryId,
    ) -> Result<ScanState, LibraryError> {
        self.require_library(caller, id).await?;
        Ok(self.libraries.scan_state(id).await?.unwrap_or(ScanState {
            library: id.clone(),
            status: ScanStatus::Idle,
            progress: 0.0,
            started_at: None,
            last_scanned_at: None,
            error: None,
        }))
    }

    async fn trigger_scan(&self, caller: &Principal, id: &LibraryId) -> Result<(), LibraryError> {
        self.require_library(caller, id).await?;
        if queue_scan(&self.libraries, &self.jobs, id, None).await? {
            Ok(())
        } else {
            Err(LibraryError::ScanInProgress)
        }
    }

    async fn unmatched(
        &self,
        caller: &Principal,
        id: &LibraryId,
        page: PageRequest,
    ) -> Result<Page<UnmatchedFile>, LibraryError> {
        self.require_library(caller, id).await?;
        Ok(self.libraries.list_unmatched(id, page).await?)
    }

    async fn duplicates(
        &self,
        caller: &Principal,
        id: &LibraryId,
        page: PageRequest,
    ) -> Result<Page<DuplicateCandidate>, LibraryError> {
        self.require_library(caller, id).await?;
        Ok(self.libraries.list_duplicates(id, page).await?)
    }

    async fn unmatched_candidates(
        &self,
        caller: &Principal,
        id: &LibraryId,
        unmatched: &UnmatchedFileId,
        query: Option<String>,
    ) -> Result<Vec<ResolveCandidate>, LibraryError> {
        let library = self.require_library(caller, id).await?;
        let file = self
            .libraries
            .get_unmatched(unmatched)
            .await?
            .filter(|file| file.library == *id)
            .ok_or(LibraryError::NotFound)?;
        let mut candidates = Vec::new();
        for candidate in &file.candidates {
            if let Some((title, year, kind)) = self.title_display(&candidate.title).await? {
                candidates.push(ResolveCandidate {
                    target: ResolveTarget::Existing(candidate.title.clone()),
                    title,
                    year,
                    kind,
                });
            }
        }
        if let Some(provider) = &self.provider {
            let parsed = parse_filename(&file.path);
            let kind = match library.kind {
                LibraryKind::Movie => MediaKind::Movie,
                LibraryKind::Tv => MediaKind::Series,
            };
            let search = MetadataQuery {
                title: query.unwrap_or(parsed.title),
                year: parsed.year,
                kind,
            };
            if let Ok(matches) = provider.search(&search).await {
                for candidate in matches {
                    candidates.push(ResolveCandidate {
                        target: ResolveTarget::Provider(candidate.external_id),
                        title: candidate.title,
                        year: candidate.year,
                        kind: candidate.kind,
                    });
                }
            }
        }
        Ok(candidates)
    }

    async fn resolve_unmatched(
        &self,
        caller: &Principal,
        id: &LibraryId,
        unmatched: &UnmatchedFileId,
        target: ResolveTarget,
    ) -> Result<(), LibraryError> {
        self.require_library(caller, id).await?;
        let file = self
            .libraries
            .get_unmatched(unmatched)
            .await?
            .filter(|file| file.library == *id)
            .ok_or(LibraryError::NotFound)?;
        let payload = IngestJobPayload {
            library: id.clone(),
            unmatched: unmatched.clone(),
            path: file.path,
            target,
        }
        .encode();
        let now = Timestamp::now();
        self.jobs
            .enqueue(queued_job(
                JobKind::Ingest,
                JobPriority::Normal,
                payload,
                None,
                now,
            ))
            .await?;
        Ok(())
    }

    async fn create_fetch(
        &self,
        caller: &Principal,
        library: &LibraryId,
        input: FetchInput,
    ) -> Result<(), LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        if !self.content_fetch_enabled {
            return Err(LibraryError::Disabled);
        }
        let target = self
            .libraries
            .get(library)
            .await?
            .ok_or(LibraryError::NotFound)?;
        if target.origin != LibraryOrigin::External {
            return Err(LibraryError::Forbidden);
        }
        let url = Url::parse(&input.source_url).map_err(|_| {
            LibraryError::InvalidRequest("source_url is not a valid URL".to_owned())
        })?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(LibraryError::InvalidRequest(
                "source_url must be an http or https URL".to_owned(),
            ));
        }
        if let Some(inspector) = &self.cookie_inspector {
            let host = url.host_str().unwrap_or_default();
            if matches!(inspector.verdict_for(host), CookieVerdict::Expired) {
                return Err(LibraryError::InvalidRequest(format!(
                    "the configured cookies for [{host}] have expired; export a fresh cookies.txt and remount it before fetching from this site"
                )));
            }
        }
        let payload = FetchJobPayload {
            library: library.clone(),
            source_url: input.source_url,
            kind: input.kind,
            title: input.title,
            year: input.year,
            external_id: input.external_id,
            season: input.season,
            episode: input.episode,
        }
        .encode();
        let now = Timestamp::now();
        self.jobs
            .enqueue(queued_job(
                JobKind::Fetch,
                JobPriority::Normal,
                payload,
                None,
                now,
            ))
            .await?;
        Ok(())
    }

    async fn dismiss_duplicate(
        &self,
        caller: &Principal,
        id: &LibraryId,
        duplicate: &DuplicateCandidateId,
    ) -> Result<(), LibraryError> {
        self.require_library(caller, id).await?;
        self.libraries
            .set_duplicate_status(duplicate, ResolutionStatus::Dismissed)
            .await?;
        Ok(())
    }

    async fn reidentify(
        &self,
        caller: &Principal,
        title: TitleRef,
        external_id: Option<ExternalId>,
        force: bool,
    ) -> Result<(), LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        self.enqueue_metadata(MetadataJobPayload::Title {
            title,
            external_id,
            force,
        })
        .await
    }

    async fn edit_movie(
        &self,
        caller: &Principal,
        id: &MovieId,
        edit: MovieEdit,
    ) -> Result<Movie, LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        let existing = self
            .catalog
            .get_movie(id)
            .await?
            .ok_or(LibraryError::NotFound)?;
        let library = super::version_library(&self.catalog, &TitleId::Movie(id.clone())).await;
        let articles = super::library_articles(&self.libraries, library).await;
        let movie = Movie {
            sort_title: sort_title(&edit.title, &articles),
            title: edit.title,
            year: edit.year,
            overview: edit.overview,
            runtime_minutes: edit.runtime_minutes,
            content_rating: edit.content_rating,
            manually_edited: true,
            updated_at: Timestamp::now(),
            ..existing
        };
        self.catalog.upsert_movie(movie.clone()).await?;
        Ok(movie)
    }

    async fn edit_series(
        &self,
        caller: &Principal,
        id: &SeriesId,
        edit: SeriesEdit,
    ) -> Result<Series, LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        let existing = self
            .catalog
            .get_series(id)
            .await?
            .ok_or(LibraryError::NotFound)?;
        let library = super::series_library(&self.catalog, id).await;
        let articles = super::library_articles(&self.libraries, library).await;
        let series = Series {
            sort_title: sort_title(&edit.title, &articles),
            title: edit.title,
            year: edit.year,
            overview: edit.overview,
            content_rating: edit.content_rating,
            manually_edited: true,
            updated_at: Timestamp::now(),
            ..existing
        };
        self.catalog.upsert_series(series.clone()).await?;
        Ok(series)
    }

    async fn edit_episode(
        &self,
        caller: &Principal,
        id: &EpisodeId,
        edit: EpisodeEdit,
    ) -> Result<Episode, LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        let existing = self
            .catalog
            .get_episode(id)
            .await?
            .ok_or(LibraryError::NotFound)?;
        let episode = Episode {
            title: edit.title,
            overview: edit.overview,
            runtime_minutes: edit.runtime_minutes,
            air_date: edit.air_date,
            manually_edited: true,
            updated_at: Timestamp::now(),
            ..existing
        };
        self.catalog.upsert_episode(episode.clone()).await?;
        Ok(episode)
    }

    async fn refresh_library_metadata(
        &self,
        caller: &Principal,
        id: &LibraryId,
    ) -> Result<(), LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        let library = self.require_library(caller, id).await?;
        let filter = TitleListFilter {
            libraries: Some(vec![id.clone()]),
            ..TitleListFilter::default()
        };
        let titles: Vec<TitleRef> = match library.kind {
            LibraryKind::Movie => self
                .catalog
                .list_movies_filtered(&filter, PageRequest::ALL)
                .await?
                .items
                .into_iter()
                .map(|movie| TitleRef::Movie(movie.id))
                .collect(),
            LibraryKind::Tv => self
                .catalog
                .list_series_filtered(&filter, PageRequest::ALL)
                .await?
                .items
                .into_iter()
                .map(|series| TitleRef::Series(series.id))
                .collect(),
        };
        for title in titles {
            self.enqueue_metadata(MetadataJobPayload::Title {
                title,
                external_id: None,
                force: false,
            })
            .await?;
        }
        Ok(())
    }

    async fn refresh_person(&self, caller: &Principal, id: &PersonId) -> Result<(), LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        self.enqueue_metadata(MetadataJobPayload::People {
            ids: vec![id.clone()],
            force: true,
        })
        .await
    }

    async fn relink_version(
        &self,
        caller: &Principal,
        version: &VersionId,
        target: ResolveTarget,
    ) -> Result<(), LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        let detail = self
            .catalog
            .version_detail(version)
            .await?
            .ok_or(LibraryError::NotFound)?;
        if !detail.version.available {
            return Err(LibraryError::Unavailable(
                "the file for this version is missing".to_owned(),
            ));
        }
        if let ResolveTarget::Existing(title) = &target {
            let exists = match title {
                TitleId::Movie(id) => self.catalog.get_movie(id).await?.is_some(),
                TitleId::Episode(id) => self.catalog.get_episode(id).await?.is_some(),
            };
            if !exists {
                return Err(LibraryError::NotFound);
            }
        }
        self.enqueue_relink(&detail.version, target).await
    }

    async fn relink_series(
        &self,
        caller: &Principal,
        series: &SeriesId,
        target: ResolveTarget,
    ) -> Result<(), LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        if matches!(target, ResolveTarget::Existing(_)) {
            return Err(LibraryError::InvalidRequest(
                "a series can only be relinked to a provider id".to_owned(),
            ));
        }
        self.catalog
            .get_series(series)
            .await?
            .ok_or(LibraryError::NotFound)?;
        let versions = self.series_versions(series).await?;
        let playable: HashSet<TitleId> = versions
            .iter()
            .filter(|version| version.available)
            .map(|version| version.title.clone())
            .collect();
        let episodes = self
            .catalog
            .episode_ids_for_series(std::slice::from_ref(series))
            .await?;
        let stranded = episodes
            .get(series)
            .map(|ids| {
                ids.iter()
                    .filter(|id| !playable.contains(&TitleId::Episode((*id).clone())))
                    .count()
            })
            .unwrap_or_default();
        if stranded > 0 {
            return Err(LibraryError::Unavailable(format!(
                "{stranded} episodes have no available file"
            )));
        }
        if versions.is_empty() {
            return Err(LibraryError::Unavailable(
                "the series has nothing to relink".to_owned(),
            ));
        }
        for version in versions {
            self.enqueue_relink(&version, target.clone()).await?;
        }
        Ok(())
    }

    async fn delete_version(
        &self,
        caller: &Principal,
        version: &VersionId,
    ) -> Result<(), LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        if self.catalog.get_version(version).await?.is_none() {
            return Err(LibraryError::NotFound);
        }
        self.catalog.delete_version(version).await?;
        Ok(())
    }

    async fn delete_movie(&self, caller: &Principal, movie: &MovieId) -> Result<(), LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        if self.catalog.get_movie(movie).await?.is_none() {
            return Err(LibraryError::NotFound);
        }
        if !self.catalog.delete_movie(movie).await? {
            return Err(LibraryError::NotEmpty(
                "the movie still has versions".to_owned(),
            ));
        }
        Ok(())
    }

    async fn delete_series(
        &self,
        caller: &Principal,
        series: &SeriesId,
    ) -> Result<(), LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        if self.catalog.get_series(series).await?.is_none() {
            return Err(LibraryError::NotFound);
        }
        if !self.catalog.delete_series(series).await? {
            return Err(LibraryError::NotEmpty(
                "the series still has seasons".to_owned(),
            ));
        }
        Ok(())
    }

    async fn delete_season(
        &self,
        caller: &Principal,
        season: &SeasonId,
    ) -> Result<(), LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        if self.catalog.get_season(season).await?.is_none() {
            return Err(LibraryError::NotFound);
        }
        if !self.catalog.delete_season(season).await? {
            return Err(LibraryError::NotEmpty(
                "the season still has episodes".to_owned(),
            ));
        }
        Ok(())
    }

    async fn delete_episode(
        &self,
        caller: &Principal,
        episode: &EpisodeId,
    ) -> Result<(), LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        if self.catalog.get_episode(episode).await?.is_none() {
            return Err(LibraryError::NotFound);
        }
        if !self.catalog.delete_episode(episode).await? {
            return Err(LibraryError::NotEmpty(
                "the episode still has versions".to_owned(),
            ));
        }
        Ok(())
    }

    async fn trigger_transcription(
        &self,
        caller: &Principal,
        version: &VersionId,
        audio_track_index: Option<u32>,
        source_language: Option<String>,
    ) -> Result<(), LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        if !self.transcription_enabled {
            return Err(LibraryError::Disabled);
        }
        let detail = self
            .catalog
            .version_detail(version)
            .await?
            .ok_or(LibraryError::NotFound)?;
        if let Some(index) = audio_track_index
            && !detail.audio.iter().any(|track| track.index == index)
        {
            return Err(LibraryError::NotFound);
        }
        let job = transcription_job(
            version,
            &detail.version.path,
            source_language,
            audio_track_index,
            true,
        );
        self.jobs.enqueue(job).await?;
        Ok(())
    }

    async fn trigger_translation(
        &self,
        caller: &Principal,
        version: &VersionId,
        source_subtitle: &SubtitleFileId,
        target_language: String,
    ) -> Result<(), LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        if !self.translation_enabled {
            return Err(LibraryError::Disabled);
        }
        let detail = self
            .catalog
            .version_detail(version)
            .await?
            .ok_or(LibraryError::NotFound)?;
        if !detail
            .subtitle_files
            .iter()
            .any(|file| file.id == *source_subtitle)
        {
            return Err(LibraryError::NotFound);
        }
        let job = translation_job_with_source(version, &source_subtitle.0, &target_language);
        self.jobs.enqueue(job).await?;
        Ok(())
    }

    async fn trigger_upscale(
        &self,
        caller: &Principal,
        version: &VersionId,
        target_height: u32,
    ) -> Result<(), LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        if !self.upscaling_enabled {
            return Err(LibraryError::Disabled);
        }
        self.catalog
            .version_detail(version)
            .await?
            .ok_or(LibraryError::NotFound)?;
        self.jobs
            .enqueue(upscale_job(version, target_height))
            .await?;
        Ok(())
    }

    async fn trigger_combine(
        &self,
        caller: &Principal,
        version: &VersionId,
        top: &SubtitleFileId,
        bottom: &SubtitleFileId,
    ) -> Result<(), LibraryError> {
        if !acl::is_admin(caller) {
            return Err(LibraryError::Forbidden);
        }
        let detail = self
            .catalog
            .version_detail(version)
            .await?
            .ok_or(LibraryError::NotFound)?;
        let has = |id: &SubtitleFileId| detail.subtitle_files.iter().any(|file| file.id == *id);
        if !has(top) || !has(bottom) {
            return Err(LibraryError::NotFound);
        }
        let job = combine_job(version, &top.0, &bottom.0);
        self.jobs.enqueue(job).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::job::Job;
    use domain::library::WatcherStrategy;
    use domain::repository::JobRepository;
    use domain::user::{Role, User, UserId};
    use jiff::Timestamp;
    use mocks::{
        MockCatalogRepo, MockJobStore, MockLibraryRepo, MockMetadataProvider, MockUserRepo,
    };

    type Svc = LibraryServiceImpl<
        MockLibraryRepo,
        MockUserRepo,
        MockJobStore,
        MockCatalogRepo,
        MockMetadataProvider,
    >;

    fn library(id: &str) -> Library {
        Library {
            id: LibraryId(id.into()),
            name: format!("Lib {id}"),
            origin: LibraryOrigin::Local,
            kind: LibraryKind::Movie,
            sort_articles: Vec::new(),
            roots: vec!["/media".into()],
            watcher: WatcherStrategy::Manual,
            scan_schedule: None,
            metadata_sources: Vec::new(),
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn user(id: &str) -> User {
        User {
            id: UserId(id.into()),
            username: id.into(),
            password_hash: "hash".into(),
            role: Role::User,
            max_content_rating: None,
            preferred_audio: Vec::new(),
            preferred_subtitle: Vec::new(),
            concurrent_stream_limit: None,
            bitrate_cap: None,
            active: true,
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
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

    fn automation() -> Principal {
        Principal {
            user: UserId("webhook".into()),
            role: Role::Automation,
        }
    }

    fn page() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 10,
        }
    }

    async fn seeded() -> Svc {
        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library("lib1"));
        libraries.insert_library(library("lib2"));
        let users = MockUserRepo::new();
        users.insert(user("u1"));
        users.insert(user("admin"));
        users
            .set_library_access(&UserId("u1".into()), &[LibraryId("lib1".into())])
            .await
            .unwrap();
        LibraryServiceImpl::new(
            libraries,
            users,
            MockJobStore::new(),
            MockCatalogRepo::new(),
            None,
        )
    }

    #[tokio::test]
    async fn admin_sees_all_member_filtered() {
        let svc = seeded().await;
        assert_eq!(svc.libraries(&admin()).await.unwrap().len(), 2);
        let visible = svc.libraries(&member()).await.unwrap();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id, LibraryId("lib1".into()));
    }

    #[tokio::test]
    async fn detail_hides_disallowed_as_not_found() {
        let svc = seeded().await;
        assert!(
            svc.library(&member(), &LibraryId("lib1".into()))
                .await
                .is_ok()
        );
        assert!(matches!(
            svc.library(&member(), &LibraryId("lib2".into()))
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
        assert!(matches!(
            svc.library(&admin(), &LibraryId("missing".into()))
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn scan_state_defaults_then_trigger_enqueues_and_conflicts() {
        let svc = seeded().await;
        let id = LibraryId("lib1".into());
        let state = svc.scan_state(&admin(), &id).await.unwrap();
        assert_eq!(state.status, ScanStatus::Idle);

        svc.trigger_scan(&admin(), &id).await.unwrap();
        assert_eq!(svc.jobs.list().await.unwrap().len(), 1);
        assert_eq!(svc.jobs.list().await.unwrap()[0].kind, JobKind::LibraryScan);
        assert_eq!(
            svc.scan_state(&admin(), &id).await.unwrap().status,
            ScanStatus::Queued
        );
        assert!(matches!(
            svc.trigger_scan(&admin(), &id).await.unwrap_err(),
            LibraryError::ScanInProgress
        ));
    }

    #[tokio::test]
    async fn automation_bypasses_acl_for_scan() {
        let svc = seeded().await;
        let id = LibraryId("lib2".into());
        svc.trigger_scan(&automation(), &id).await.unwrap();
        assert_eq!(
            svc.scan_state(&automation(), &id).await.unwrap().status,
            ScanStatus::Queued
        );
    }

    #[tokio::test]
    async fn unmatched_and_duplicates_gated() {
        let svc = seeded().await;
        let id = LibraryId("lib1".into());
        assert!(
            svc.unmatched(&admin(), &id, page())
                .await
                .unwrap()
                .items
                .is_empty()
        );
        assert!(
            svc.duplicates(&admin(), &id, page())
                .await
                .unwrap()
                .items
                .is_empty()
        );
        assert!(matches!(
            svc.unmatched(&member(), &LibraryId("lib2".into()), page())
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
        assert!(matches!(
            svc.duplicates(&member(), &LibraryId("lib2".into()), page())
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn backend_errors_propagate() {
        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library("lib1"));
        libraries.set_fail_get();
        let svc = LibraryServiceImpl::new(
            libraries,
            MockUserRepo::new(),
            MockJobStore::new(),
            MockCatalogRepo::new(),
            None::<MockMetadataProvider>,
        );
        assert!(matches!(
            svc.library(&admin(), &LibraryId("lib1".into()))
                .await
                .unwrap_err(),
            LibraryError::Repository(_)
        ));
    }

    // Everything here opens by resolving the library. A method absent from the list
    // has an unread repository error that could just as easily surface as NotFound,
    // which reads to a caller as "this library is gone" rather than "try again".
    #[tokio::test]
    async fn every_library_scoped_method_surfaces_a_backend_error() {
        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library("lib1"));
        libraries.set_fail_get();
        let svc = LibraryServiceImpl::new(
            libraries,
            MockUserRepo::new(),
            MockJobStore::new(),
            MockCatalogRepo::new(),
            None::<MockMetadataProvider>,
        )
        .with_content_fetch(true);

        let id = LibraryId("lib1".into());
        let page = PageRequest {
            offset: 0,
            limit: 10,
        };
        let unmatched = UnmatchedFileId("uf1".into());
        let duplicate = DuplicateCandidateId("d1".into());

        macro_rules! is_repository_error {
            ($call:expr) => {
                assert!(matches!(
                    $call.await.unwrap_err(),
                    LibraryError::Repository(_)
                ))
            };
        }

        is_repository_error!(svc.library(&admin(), &id));
        is_repository_error!(svc.scan_state(&admin(), &id));
        is_repository_error!(svc.trigger_scan(&admin(), &id));
        is_repository_error!(svc.unmatched(&admin(), &id, page));
        is_repository_error!(svc.duplicates(&admin(), &id, page));
        is_repository_error!(svc.unmatched_candidates(&admin(), &id, &unmatched, None));
        is_repository_error!(svc.resolve_unmatched(
            &admin(),
            &id,
            &unmatched,
            ResolveTarget::Existing(TitleId::Movie(MovieId("m1".into())))
        ));
        is_repository_error!(svc.dismiss_duplicate(&admin(), &id, &duplicate));
        is_repository_error!(svc.refresh_library_metadata(&admin(), &id));
        is_repository_error!(svc.create_fetch(&admin(), &id, fetch_input()));
        is_repository_error!(svc.update_library(&admin(), &id, library_update()));
        is_repository_error!(svc.delete_library(&admin(), &id));
    }

    // The member path reads grants before it reads the library, so its error arm is
    // a different one, and a swallowed failure there reads as "no access" instead.
    #[tokio::test]
    async fn a_member_whose_grants_cannot_be_read_is_refused_with_the_real_error() {
        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library("lib1"));
        let users = MockUserRepo::new();
        users.set_fail();
        let svc = LibraryServiceImpl::new(
            libraries,
            users,
            MockJobStore::new(),
            MockCatalogRepo::new(),
            None::<MockMetadataProvider>,
        );

        assert!(matches!(
            svc.libraries(&member()).await.unwrap_err(),
            LibraryError::Repository(_)
        ));
        assert!(matches!(
            svc.library(&member(), &LibraryId("lib1".into()))
                .await
                .unwrap_err(),
            LibraryError::Repository(_)
        ));
    }

    fn new_library() -> NewLibrary {
        NewLibrary {
            name: "New".into(),
            kind: LibraryKind::Movie,
            origin: LibraryOrigin::Local,
            roots: vec!["/n".into()],
            sort_articles: vec!["the".into()],
            watcher: WatcherStrategy::Manual,
            scan_schedule: None,
            metadata_sources: vec!["tmdb".into()],
        }
    }

    fn library_update() -> LibraryUpdate {
        LibraryUpdate {
            name: "Renamed".into(),
            kind: LibraryKind::Tv,
            roots: vec!["/n".into()],
            sort_articles: vec!["die".into()],
            watcher: WatcherStrategy::Manual,
            scan_schedule: None,
            metadata_sources: Vec::new(),
        }
    }

    #[tokio::test]
    async fn deleting_a_library_takes_its_content_out_of_service() {
        use domain::catalog::{MovieId, TitleId, VersionId};

        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library("lib1"));
        libraries.insert_library(library("lib2"));
        let users = MockUserRepo::new();
        users.insert(user("u1"));
        users
            .set_library_access(
                &UserId("u1".into()),
                &[LibraryId("lib1".into()), LibraryId("lib2".into())],
            )
            .await
            .unwrap();
        let catalog = MockCatalogRepo::new();
        let title = TitleId::Movie(MovieId("m1".into()));
        catalog.add_version(refresh_version("doomed", title.clone(), "lib1"));
        catalog.add_version(refresh_version("spared", title, "lib2"));
        let svc = LibraryServiceImpl::new(
            libraries,
            users.clone(),
            MockJobStore::new(),
            catalog.clone(),
            None::<MockMetadataProvider>,
        );

        svc.delete_library(&admin(), &LibraryId("lib1".into()))
            .await
            .unwrap();

        let doomed = catalog
            .get_version(&VersionId("doomed".into()))
            .await
            .unwrap()
            .unwrap();
        let spared = catalog
            .get_version(&VersionId("spared".into()))
            .await
            .unwrap()
            .unwrap();
        assert!(!doomed.available, "the dead library's content stays live");
        assert!(spared.available, "another library was caught in the blast");

        let access = users
            .list_library_access(&UserId("u1".into()))
            .await
            .unwrap();
        let granted: Vec<String> = access.into_iter().map(|a| a.library.0).collect();
        assert_eq!(granted, vec!["lib2".to_owned()]);
    }

    #[tokio::test]
    async fn creating_a_library_grants_its_creator_access() {
        let svc = seeded().await;

        let created = svc.create_library(&admin(), new_library()).await.unwrap();
        let granted: Vec<LibraryId> = svc
            .users
            .list_library_access(&admin().user)
            .await
            .unwrap()
            .into_iter()
            .map(|entry| entry.library)
            .collect();

        assert!(
            granted.contains(&created.id),
            "without this the library is invisible to the person who just added it"
        );
    }

    #[tokio::test]
    async fn a_creator_with_no_account_is_not_granted_anything() {
        let svc = seeded().await;
        let synthetic = Principal {
            user: UserId("bootstrap".into()),
            role: Role::Admin,
        };

        let created = svc.create_library(&synthetic, new_library()).await;

        assert!(
            created.is_ok(),
            "bootstrap creates libraries before users exist, and the access row \
             has a foreign key onto users, so granting one would fail the create"
        );
        assert!(
            svc.users
                .list_library_access(&synthetic.user)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn creating_a_second_library_keeps_the_first_grant() {
        let svc = seeded().await;

        let first = svc.create_library(&admin(), new_library()).await.unwrap();
        let second = svc.create_library(&admin(), new_library()).await.unwrap();
        let granted: Vec<LibraryId> = svc
            .users
            .list_library_access(&admin().user)
            .await
            .unwrap()
            .into_iter()
            .map(|entry| entry.library)
            .collect();

        assert!(
            granted.contains(&first.id) && granted.contains(&second.id),
            "set_library_access replaces the whole set, so the grant has to append"
        );
    }

    #[tokio::test]
    async fn create_update_delete_library() {
        let svc = seeded().await;

        let created = svc.create_library(&admin(), new_library()).await.unwrap();
        assert!(svc.library(&admin(), &created.id).await.is_ok());
        assert!(matches!(
            svc.create_library(&member(), new_library())
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));

        let updated = svc
            .update_library(&admin(), &created.id, library_update())
            .await
            .unwrap();
        assert_eq!(updated.name, "Renamed");
        assert_eq!(updated.kind, LibraryKind::Tv);
        assert!(matches!(
            svc.update_library(&admin(), &LibraryId("ghost".into()), library_update())
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
        assert!(matches!(
            svc.update_library(&member(), &created.id, library_update())
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));

        svc.delete_library(&admin(), &created.id).await.unwrap();
        assert!(matches!(
            svc.library(&admin(), &created.id).await.unwrap_err(),
            LibraryError::NotFound
        ));
        assert!(matches!(
            svc.delete_library(&admin(), &created.id).await.unwrap_err(),
            LibraryError::NotFound
        ));
        assert!(matches!(
            svc.delete_library(&member(), &LibraryId("lib1".into()))
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));
    }

    #[tokio::test]
    async fn library_writes_propagate_backend_errors() {
        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library("lib1"));
        libraries.set_fail_save();
        let svc = LibraryServiceImpl::new(
            libraries,
            MockUserRepo::new(),
            MockJobStore::new(),
            MockCatalogRepo::new(),
            None::<MockMetadataProvider>,
        );
        assert!(matches!(
            svc.create_library(&admin(), new_library())
                .await
                .unwrap_err(),
            LibraryError::Repository(_)
        ));
        assert!(matches!(
            svc.delete_library(&admin(), &LibraryId("lib1".into()))
                .await
                .unwrap_err(),
            LibraryError::Repository(_)
        ));
    }

    #[tokio::test]
    async fn unmatched_candidates_lists_catalog_and_provider() {
        use domain::catalog::{Episode, EpisodeId, Movie, MovieId, SeasonId};
        use domain::library::{MatchCandidate, UnmatchedFileId};
        use domain::metadata::{ExternalId, MetadataMatch};

        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library("lib1"));
        let uid = UnmatchedFileId("uf1".into());
        libraries
            .insert_unmatched(UnmatchedFile {
                id: uid.clone(),
                library: LibraryId("lib1".into()),
                path: "/media/The Matrix (1999).mkv".into(),
                candidates: vec![
                    MatchCandidate {
                        title: TitleId::Movie(MovieId("m1".into())),
                        confidence: 0.9,
                        label: "Alpha".into(),
                    },
                    MatchCandidate {
                        title: TitleId::Episode(EpisodeId("e1".into())),
                        confidence: 0.5,
                        label: "Beta".into(),
                    },
                ],
                created_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .unwrap();

        let catalog = MockCatalogRepo::new();
        catalog.add_movie(Movie {
            id: MovieId("m1".into()),
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
        catalog.add_episode(Episode {
            id: EpisodeId("e1".into()),
            season: SeasonId("se1".into()),
            number: 1,
            title: "Pilot".into(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        let provider = MockMetadataProvider::with_matches(vec![MetadataMatch {
            external_id: ExternalId {
                source: "tmdb".into(),
                value: "movie/603".into(),
            },
            title: "The Matrix".into(),
            year: Some(1999),
            kind: MediaKind::Movie,
        }]);

        let svc = LibraryServiceImpl::new(
            libraries,
            MockUserRepo::new(),
            MockJobStore::new(),
            catalog,
            Some(provider),
        );
        let id = LibraryId("lib1".into());
        let candidates = svc
            .unmatched_candidates(&admin(), &id, &uid, None)
            .await
            .unwrap();
        assert_eq!(candidates.len(), 3);
        assert!(matches!(candidates[0].target, ResolveTarget::Existing(_)));
        assert_eq!(candidates[0].title, "The Matrix");
        assert_eq!(candidates[0].kind, MediaKind::Movie);
        assert_eq!(candidates[1].kind, MediaKind::Series);
        assert!(matches!(candidates[2].target, ResolveTarget::Provider(_)));

        assert!(matches!(
            svc.unmatched_candidates(&admin(), &id, &UnmatchedFileId("ghost".into()), None)
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn resolve_unmatched_enqueues_ingest_job() {
        use domain::catalog::MovieId;
        use domain::library::{UnmatchedFile, UnmatchedFileId};

        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library("lib1"));
        let uid = UnmatchedFileId("uf1".into());
        libraries
            .insert_unmatched(UnmatchedFile {
                id: uid.clone(),
                library: LibraryId("lib1".into()),
                path: "/m/x.mkv".into(),
                candidates: Vec::new(),
                created_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .unwrap();
        let jobs = MockJobStore::new();
        let svc = LibraryServiceImpl::new(
            libraries,
            MockUserRepo::new(),
            jobs.clone(),
            MockCatalogRepo::new(),
            None::<MockMetadataProvider>,
        );
        let id = LibraryId("lib1".into());
        let target = ResolveTarget::Existing(TitleId::Movie(MovieId("m1".into())));

        svc.resolve_unmatched(&admin(), &id, &uid, target.clone())
            .await
            .unwrap();
        let enqueued = jobs.list().await.unwrap();
        assert_eq!(enqueued.len(), 1);
        assert_eq!(enqueued[0].kind, JobKind::Ingest);

        assert!(matches!(
            svc.resolve_unmatched(&admin(), &id, &UnmatchedFileId("ghost".into()), target)
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    fn external_library(id: &str) -> Library {
        let mut lib = library(id);
        lib.origin = LibraryOrigin::External;
        lib
    }

    fn fetch_input() -> FetchInput {
        FetchInput {
            source_url: "https://example.com/watch?v=abc".into(),
            kind: LibraryKind::Movie,
            title: "The Matrix".into(),
            year: Some(1999),
            external_id: Some("tt0133093".into()),
            season: None,
            episode: None,
        }
    }

    fn fetch_svc(jobs: MockJobStore, enabled: bool) -> Svc {
        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library("local"));
        libraries.insert_library(external_library("ext"));
        LibraryServiceImpl::new(
            libraries,
            MockUserRepo::new(),
            jobs,
            MockCatalogRepo::new(),
            None::<MockMetadataProvider>,
        )
        .with_content_fetch(enabled)
    }

    struct StubCookies(CookieVerdict);

    impl CookieInspector for StubCookies {
        fn verdict_for(&self, _host: &str) -> CookieVerdict {
            self.0
        }
    }

    fn stub_cookies(verdict: CookieVerdict) -> Option<Arc<dyn CookieInspector>> {
        Some(Arc::new(StubCookies(verdict)) as Arc<dyn CookieInspector>)
    }

    #[tokio::test]
    async fn create_fetch_rejects_expired_cookies() {
        let jobs = MockJobStore::new();
        let svc = fetch_svc(jobs.clone(), true)
            .with_cookie_inspector(stub_cookies(CookieVerdict::Expired));
        assert!(matches!(
            svc.create_fetch(&admin(), &LibraryId("ext".into()), fetch_input())
                .await
                .unwrap_err(),
            LibraryError::InvalidRequest(_)
        ));
        assert!(jobs.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn create_fetch_proceeds_when_cookies_live() {
        let jobs = MockJobStore::new();
        let svc =
            fetch_svc(jobs.clone(), true).with_cookie_inspector(stub_cookies(CookieVerdict::Live));
        svc.create_fetch(&admin(), &LibraryId("ext".into()), fetch_input())
            .await
            .unwrap();
        assert_eq!(jobs.list().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn create_fetch_proceeds_when_cookies_not_applicable() {
        let jobs = MockJobStore::new();
        let svc = fetch_svc(jobs.clone(), true)
            .with_cookie_inspector(stub_cookies(CookieVerdict::NotApplicable));
        svc.create_fetch(&admin(), &LibraryId("ext".into()), fetch_input())
            .await
            .unwrap();
        assert_eq!(jobs.list().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn create_fetch_enqueues_fetch_job_for_external_library() {
        let jobs = MockJobStore::new();
        let svc = fetch_svc(jobs.clone(), true);
        svc.create_fetch(&admin(), &LibraryId("ext".into()), fetch_input())
            .await
            .unwrap();
        let enqueued = jobs.list().await.unwrap();
        assert_eq!(enqueued.len(), 1);
        assert_eq!(enqueued[0].kind, JobKind::Fetch);
    }

    #[tokio::test]
    async fn create_fetch_requires_admin() {
        let svc = fetch_svc(MockJobStore::new(), true);
        assert!(matches!(
            svc.create_fetch(&member(), &LibraryId("ext".into()), fetch_input())
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));
    }

    #[tokio::test]
    async fn create_fetch_is_disabled_when_flag_off() {
        let svc = fetch_svc(MockJobStore::new(), false);
        assert!(matches!(
            svc.create_fetch(&admin(), &LibraryId("ext".into()), fetch_input())
                .await
                .unwrap_err(),
            LibraryError::Disabled
        ));
    }

    #[tokio::test]
    async fn create_fetch_rejects_local_library() {
        let svc = fetch_svc(MockJobStore::new(), true);
        assert!(matches!(
            svc.create_fetch(&admin(), &LibraryId("local".into()), fetch_input())
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));
    }

    #[tokio::test]
    async fn create_fetch_missing_library_is_not_found() {
        let svc = fetch_svc(MockJobStore::new(), true);
        assert!(matches!(
            svc.create_fetch(&admin(), &LibraryId("ghost".into()), fetch_input())
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn create_fetch_rejects_non_http_scheme() {
        let jobs = MockJobStore::new();
        let svc = fetch_svc(jobs.clone(), true);
        let mut input = fetch_input();
        input.source_url = "file:///etc/passwd".into();
        assert!(matches!(
            svc.create_fetch(&admin(), &LibraryId("ext".into()), input)
                .await
                .unwrap_err(),
            LibraryError::InvalidRequest(_)
        ));
        assert!(jobs.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn create_fetch_rejects_unparseable_url() {
        let svc = fetch_svc(MockJobStore::new(), true);
        let mut input = fetch_input();
        input.source_url = "not a url".into();
        assert!(matches!(
            svc.create_fetch(&admin(), &LibraryId("ext".into()), input)
                .await
                .unwrap_err(),
            LibraryError::InvalidRequest(_)
        ));
    }

    #[tokio::test]
    async fn dismissing_duplicates_hides_them() {
        use domain::catalog::MovieId;
        use domain::library::{DuplicateCandidate, DuplicateCandidateId};

        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library("lib1"));
        let id = LibraryId("lib1".into());
        let dup = |d: &str| DuplicateCandidate {
            id: DuplicateCandidateId(d.into()),
            title: TitleId::Movie(MovieId("m1".into())),
            paths: vec!["/a.mkv".into()],
        };
        libraries.insert_duplicate(&id, dup("d1")).await.unwrap();
        libraries.insert_duplicate(&id, dup("d2")).await.unwrap();
        let svc = LibraryServiceImpl::new(
            libraries,
            MockUserRepo::new(),
            MockJobStore::new(),
            MockCatalogRepo::new(),
            None::<MockMetadataProvider>,
        );

        svc.dismiss_duplicate(&admin(), &id, &DuplicateCandidateId("d1".into()))
            .await
            .unwrap();
        svc.dismiss_duplicate(&admin(), &id, &DuplicateCandidateId("d2".into()))
            .await
            .unwrap();
        assert!(
            svc.duplicates(&admin(), &id, page())
                .await
                .unwrap()
                .items
                .is_empty()
        );

        assert!(matches!(
            svc.dismiss_duplicate(
                &admin(),
                &LibraryId("ghost".into()),
                &DuplicateCandidateId("d1".into())
            )
            .await
            .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn reidentify_enqueues_metadata_job_admin_only() {
        use domain::catalog::MovieId;
        use domain::metadata::ExternalId;

        let jobs = MockJobStore::new();
        let svc = LibraryServiceImpl::new(
            MockLibraryRepo::new(),
            MockUserRepo::new(),
            jobs.clone(),
            MockCatalogRepo::new(),
            None::<MockMetadataProvider>,
        );
        let title = TitleRef::Movie(MovieId("m1".into()));

        svc.reidentify(
            &admin(),
            title.clone(),
            Some(ExternalId {
                source: "tmdb".into(),
                value: "movie/603".into(),
            }),
            false,
        )
        .await
        .unwrap();
        let enqueued = jobs.list().await.unwrap();
        assert_eq!(enqueued.len(), 1);
        assert_eq!(enqueued[0].kind, JobKind::Metadata);

        assert!(matches!(
            svc.reidentify(&member(), title, None, false)
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));
    }

    #[tokio::test]
    async fn refresh_person_enqueues_metadata_job_admin_only() {
        let jobs = MockJobStore::new();
        let svc = LibraryServiceImpl::new(
            MockLibraryRepo::new(),
            MockUserRepo::new(),
            jobs.clone(),
            MockCatalogRepo::new(),
            None::<MockMetadataProvider>,
        );

        svc.refresh_person(&admin(), &PersonId("p1".into()))
            .await
            .unwrap();
        let enqueued = jobs.list().await.unwrap();
        assert_eq!(enqueued.len(), 1);
        assert_eq!(enqueued[0].kind, JobKind::Metadata);

        assert!(matches!(
            svc.refresh_person(&member(), &PersonId("p1".into()))
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));
    }

    fn refresh_version(id: &str, title: TitleId, library: &str) -> domain::catalog::Version {
        use domain::catalog::{Version, VersionId};
        use domain::common::Quality;
        Version {
            id: VersionId(id.into()),
            title,
            library: LibraryId(library.into()),
            quality: Quality::Fhd,
            container: "mkv".into(),
            path: format!("/media/{id}.mkv"),
            size_bytes: 1,
            duration_ms: 1000,
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn refresh_movie_row(id: &str) -> domain::catalog::Movie {
        use domain::catalog::{Movie, MovieId};
        Movie {
            id: MovieId(id.into()),
            title: id.into(),
            sort_title: id.into(),
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

    fn refresh_catalog() -> MockCatalogRepo {
        use domain::catalog::{
            Episode, EpisodeId, MovieId, Season, SeasonId, Series, SeriesId, TitleId,
        };
        let catalog = MockCatalogRepo::new();
        for id in ["m1", "m2", "elsewhere"] {
            catalog.add_movie(refresh_movie_row(id));
        }
        catalog.add_version(refresh_version(
            "v1",
            TitleId::Movie(MovieId("m1".into())),
            "films",
        ));
        catalog.add_version(refresh_version(
            "v2",
            TitleId::Movie(MovieId("m2".into())),
            "films",
        ));
        catalog.add_version(refresh_version(
            "v3",
            TitleId::Movie(MovieId("elsewhere".into())),
            "other",
        ));
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
        for episode in ["e1", "e2"] {
            catalog.add_episode(Episode {
                id: EpisodeId(episode.into()),
                season: SeasonId("se1".into()),
                number: 1,
                title: episode.into(),
                overview: None,
                runtime_minutes: None,
                air_date: None,
                manually_edited: false,
                added_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
                artwork: Vec::new(),
            });
            catalog.add_version(refresh_version(
                episode,
                TitleId::Episode(EpisodeId(episode.into())),
                "shows",
            ));
        }
        catalog
    }

    fn refresh_svc(catalog: MockCatalogRepo) -> (Svc, MockJobStore) {
        let libraries = MockLibraryRepo::new();
        libraries.insert_library(library("films"));
        libraries.insert_library(library("other"));
        let mut shows = library("shows");
        shows.kind = LibraryKind::Tv;
        libraries.insert_library(shows);
        let jobs = MockJobStore::new();
        let svc = LibraryServiceImpl::new(
            libraries,
            MockUserRepo::new(),
            jobs.clone(),
            catalog,
            None::<MockMetadataProvider>,
        );
        (svc, jobs)
    }

    fn refreshed_titles(jobs: &[Job]) -> Vec<TitleRef> {
        jobs.iter()
            .map(
                |job| match MetadataJobPayload::decode(&job.payload).unwrap() {
                    MetadataJobPayload::Title {
                        title,
                        external_id: None,
                        force: false,
                    } => title,
                    other => panic!("unexpected metadata payload {other:?}"),
                },
            )
            .collect()
    }

    #[tokio::test]
    async fn refreshing_a_movie_library_queues_one_metadata_job_per_movie_in_it() {
        use domain::catalog::MovieId;
        let (svc, jobs) = refresh_svc(refresh_catalog());
        svc.refresh_library_metadata(&admin(), &LibraryId("films".into()))
            .await
            .unwrap();
        let enqueued = jobs.list().await.unwrap();
        assert!(enqueued.iter().all(|job| job.kind == JobKind::Metadata));
        let mut titles = refreshed_titles(&enqueued);
        titles.sort_by_key(|title| title.id().to_string());
        assert_eq!(
            titles,
            vec![
                TitleRef::Movie(MovieId("m1".into())),
                TitleRef::Movie(MovieId("m2".into())),
            ]
        );
    }

    #[tokio::test]
    async fn refreshing_a_tv_library_queues_the_series_not_its_episodes() {
        use domain::catalog::SeriesId;
        let (svc, jobs) = refresh_svc(refresh_catalog());
        svc.refresh_library_metadata(&admin(), &LibraryId("shows".into()))
            .await
            .unwrap();
        let enqueued = jobs.list().await.unwrap();
        assert_eq!(
            refreshed_titles(&enqueued),
            vec![TitleRef::Series(SeriesId("s1".into()))]
        );
    }

    #[tokio::test]
    async fn refreshing_a_library_never_queues_a_scan() {
        let (svc, jobs) = refresh_svc(refresh_catalog());
        svc.refresh_library_metadata(&admin(), &LibraryId("films".into()))
            .await
            .unwrap();
        assert!(
            jobs.list()
                .await
                .unwrap()
                .iter()
                .all(|job| job.kind != JobKind::LibraryScan)
        );
        assert_eq!(
            svc.scan_state(&admin(), &LibraryId("films".into()))
                .await
                .unwrap()
                .status,
            ScanStatus::Idle
        );
    }

    #[tokio::test]
    async fn refreshing_a_library_is_admin_only_and_needs_the_library_to_exist() {
        let (svc, jobs) = refresh_svc(refresh_catalog());
        assert!(matches!(
            svc.refresh_library_metadata(&member(), &LibraryId("films".into()))
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));
        assert!(matches!(
            svc.refresh_library_metadata(&admin(), &LibraryId("ghost".into()))
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
        assert!(jobs.list().await.unwrap().is_empty());
    }

    fn seeded_version_catalog() -> MockCatalogRepo {
        use domain::catalog::{Movie, MovieId, TitleId, Version, VersionId};
        use domain::common::Quality;
        let catalog = MockCatalogRepo::new();
        catalog.add_version(Version {
            id: VersionId("v1".into()),
            title: TitleId::Movie(MovieId("m-old".into())),
            library: LibraryId("lib1".into()),
            quality: Quality::Fhd,
            container: "mkv".into(),
            path: "/media/v1.mkv".into(),
            size_bytes: 1,
            duration_ms: 1000,
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        });
        catalog.add_movie(Movie {
            id: MovieId("m-new".into()),
            title: "New".into(),
            sort_title: "new".into(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog
    }

    fn relink_svc(catalog: MockCatalogRepo) -> (Svc, MockJobStore) {
        let jobs = MockJobStore::new();
        let svc = LibraryServiceImpl::new(
            MockLibraryRepo::new(),
            MockUserRepo::new(),
            jobs.clone(),
            catalog,
            None::<MockMetadataProvider>,
        );
        (svc, jobs)
    }

    #[tokio::test]
    async fn relink_existing_movie_enqueues_job_admin_only() {
        use domain::catalog::{MovieId, TitleId, VersionId};
        let (svc, jobs) = relink_svc(seeded_version_catalog());
        let target = ResolveTarget::Existing(TitleId::Movie(MovieId("m-new".into())));
        svc.relink_version(&admin(), &VersionId("v1".into()), target.clone())
            .await
            .unwrap();
        let enqueued = jobs.list().await.unwrap();
        assert_eq!(enqueued.len(), 1);
        assert_eq!(enqueued[0].kind, JobKind::Relink);
        assert!(matches!(
            svc.relink_version(&member(), &VersionId("v1".into()), target)
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));
    }

    #[tokio::test]
    async fn relink_to_existing_episode_enqueues() {
        use domain::catalog::{Episode, EpisodeId, SeasonId, TitleId, VersionId};
        let catalog = seeded_version_catalog();
        catalog.add_episode(Episode {
            id: EpisodeId("e-new".into()),
            season: SeasonId("s1".into()),
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
        let (svc, jobs) = relink_svc(catalog);
        svc.relink_version(
            &admin(),
            &VersionId("v1".into()),
            ResolveTarget::Existing(TitleId::Episode(EpisodeId("e-new".into()))),
        )
        .await
        .unwrap();
        assert_eq!(jobs.list().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn relink_provider_target_enqueues() {
        use domain::catalog::VersionId;
        use domain::metadata::ExternalId;
        let (svc, jobs) = relink_svc(seeded_version_catalog());
        svc.relink_version(
            &admin(),
            &VersionId("v1".into()),
            ResolveTarget::Provider(ExternalId {
                source: "tmdb".into(),
                value: "movie/603".into(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(jobs.list().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn relink_series_fans_out_over_every_episode_version() {
        use domain::catalog::{
            Episode, EpisodeId, Season, SeasonId, Series, SeriesId, TitleId, Version, VersionId,
        };
        use domain::common::Quality;
        use domain::metadata::ExternalId;

        let catalog = MockCatalogRepo::new();
        catalog.add_series(Series {
            id: SeriesId("sh1".into()),
            title: "Stargate SG-1".into(),
            sort_title: "stargate sg-1".into(),
            year: Some(1997),
            overview: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        for (season_no, season_id) in [(1u16, "se1"), (2, "se2")] {
            catalog.add_season(Season {
                id: SeasonId(season_id.into()),
                series: SeriesId("sh1".into()),
                number: season_no,
                title: None,
                overview: None,
                added_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
                artwork: Vec::new(),
            });
            for episode_no in 1u16..=2 {
                let episode = format!("{season_id}e{episode_no}");
                catalog.add_episode(Episode {
                    id: EpisodeId(episode.clone()),
                    season: SeasonId(season_id.into()),
                    number: episode_no,
                    title: format!("Episode {episode_no}"),
                    overview: None,
                    runtime_minutes: None,
                    air_date: None,
                    manually_edited: false,
                    added_at: Timestamp::UNIX_EPOCH,
                    updated_at: Timestamp::UNIX_EPOCH,
                    artwork: Vec::new(),
                });
                catalog.add_version(Version {
                    id: VersionId(format!("v-{episode}")),
                    title: TitleId::Episode(EpisodeId(episode.clone())),
                    library: LibraryId("lib".into()),
                    quality: Quality::Hd,
                    container: "mkv".into(),
                    path: format!("/tv/{episode}.mkv"),
                    size_bytes: 1,
                    duration_ms: 1,
                    available: true,
                    added_at: Timestamp::UNIX_EPOCH,
                    updated_at: Timestamp::UNIX_EPOCH,
                });
            }
        }
        let (svc, jobs) = relink_svc(catalog);
        let target = ResolveTarget::Provider(ExternalId {
            source: "tmdb".into(),
            value: "tv/4629".into(),
        });

        svc.relink_series(&admin(), &SeriesId("sh1".into()), target.clone())
            .await
            .unwrap();

        let enqueued = jobs.list().await.unwrap();
        assert_eq!(
            enqueued.len(),
            4,
            "one relink per version across every season, not one for the show"
        );
        assert!(enqueued.iter().all(|job| job.kind == JobKind::Relink));

        assert!(matches!(
            svc.relink_series(&member(), &SeriesId("sh1".into()), target)
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));
    }

    fn tree_catalog() -> MockCatalogRepo {
        use domain::catalog::{
            Episode, EpisodeId, Movie, MovieId, Season, SeasonId, Series, SeriesId, TitleId,
            Version, VersionId,
        };
        use domain::common::Quality;
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(Movie {
            id: MovieId("m1".into()),
            title: "Movie".into(),
            sort_title: "movie".into(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
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
            title: "Episode".into(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        for (id, title) in [
            ("v1", TitleId::Movie(MovieId("m1".into()))),
            ("v2", TitleId::Episode(EpisodeId("e1".into()))),
        ] {
            catalog.add_version(Version {
                id: VersionId(id.into()),
                title,
                library: LibraryId("lib1".into()),
                quality: Quality::Hd,
                container: "mkv".into(),
                path: format!("/media/{id}.mkv"),
                size_bytes: 1,
                duration_ms: 1,
                available: true,
                added_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
            });
        }
        catalog
    }

    #[tokio::test]
    async fn deleting_a_title_is_admin_only() {
        use domain::catalog::{EpisodeId, MovieId, SeasonId, SeriesId};
        let (svc, _jobs) = relink_svc(tree_catalog());
        assert!(matches!(
            svc.delete_movie(&member(), &MovieId("m1".into()))
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));
        assert!(matches!(
            svc.delete_series(&member(), &SeriesId("s1".into()))
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));
        assert!(matches!(
            svc.delete_season(&member(), &SeasonId("se1".into()))
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));
        assert!(matches!(
            svc.delete_episode(&member(), &EpisodeId("e1".into()))
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));
    }

    #[tokio::test]
    async fn deleting_an_unknown_title_is_not_found() {
        use domain::catalog::{EpisodeId, MovieId, SeasonId, SeriesId};
        let (svc, _jobs) = relink_svc(MockCatalogRepo::new());
        assert!(matches!(
            svc.delete_movie(&admin(), &MovieId("ghost".into()))
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
        assert!(matches!(
            svc.delete_series(&admin(), &SeriesId("ghost".into()))
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
        assert!(matches!(
            svc.delete_season(&admin(), &SeasonId("ghost".into()))
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
        assert!(matches!(
            svc.delete_episode(&admin(), &EpisodeId("ghost".into()))
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn a_title_is_deletable_only_after_its_children_are_gone() {
        use domain::catalog::{EpisodeId, MovieId, SeasonId, SeriesId, VersionId};
        let catalog = tree_catalog();
        let (svc, _jobs) = relink_svc(catalog.clone());

        for outcome in [
            svc.delete_movie(&admin(), &MovieId("m1".into())).await,
            svc.delete_series(&admin(), &SeriesId("s1".into())).await,
            svc.delete_season(&admin(), &SeasonId("se1".into())).await,
            svc.delete_episode(&admin(), &EpisodeId("e1".into())).await,
        ] {
            assert!(
                matches!(outcome, Err(LibraryError::NotEmpty(_))),
                "a title with children is refused rather than silently kept"
            );
        }

        svc.delete_version(&admin(), &VersionId("v1".into()))
            .await
            .unwrap();
        svc.delete_version(&admin(), &VersionId("v2".into()))
            .await
            .unwrap();

        svc.delete_movie(&admin(), &MovieId("m1".into()))
            .await
            .unwrap();
        svc.delete_episode(&admin(), &EpisodeId("e1".into()))
            .await
            .unwrap();
        svc.delete_season(&admin(), &SeasonId("se1".into()))
            .await
            .unwrap();
        svc.delete_series(&admin(), &SeriesId("s1".into()))
            .await
            .unwrap();

        assert!(
            catalog
                .get_series(&SeriesId("s1".into()))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn relink_refuses_a_version_whose_file_is_missing() {
        use domain::catalog::{Movie, MovieId, TitleId, Version, VersionId};
        use domain::common::Quality;
        let catalog = MockCatalogRepo::new();
        catalog.add_version(Version {
            id: VersionId("v1".into()),
            title: TitleId::Movie(MovieId("m-old".into())),
            library: LibraryId("lib1".into()),
            quality: Quality::Fhd,
            container: "mkv".into(),
            path: "/media/gone.mkv".into(),
            size_bytes: 1,
            duration_ms: 1,
            available: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        });
        catalog.add_movie(Movie {
            id: MovieId("m-new".into()),
            title: "New".into(),
            sort_title: "new".into(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        let (svc, jobs) = relink_svc(catalog);

        let outcome = svc
            .relink_version(
                &admin(),
                &VersionId("v1".into()),
                ResolveTarget::Existing(TitleId::Movie(MovieId("m-new".into()))),
            )
            .await;

        assert!(matches!(outcome, Err(LibraryError::Unavailable(_))));
        assert!(
            jobs.list().await.unwrap().is_empty(),
            "nothing is queued for a file the probe cannot read"
        );
    }

    #[tokio::test]
    async fn relink_series_refuses_while_any_episode_has_no_available_file() {
        use domain::catalog::{
            Episode, EpisodeId, Season, SeasonId, Series, SeriesId, TitleId, Version, VersionId,
        };
        use domain::common::Quality;
        use domain::metadata::ExternalId;

        let catalog = MockCatalogRepo::new();
        catalog.add_series(Series {
            id: SeriesId("sh1".into()),
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
        catalog.add_season(Season {
            id: SeasonId("se1".into()),
            series: SeriesId("sh1".into()),
            number: 1,
            title: None,
            overview: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        for id in ["e1", "e2"] {
            catalog.add_episode(Episode {
                id: EpisodeId(id.into()),
                season: SeasonId("se1".into()),
                number: 1,
                title: id.into(),
                overview: None,
                runtime_minutes: None,
                air_date: None,
                manually_edited: false,
                added_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
                artwork: Vec::new(),
            });
        }
        catalog.add_version(Version {
            id: VersionId("v1".into()),
            title: TitleId::Episode(EpisodeId("e1".into())),
            library: LibraryId("lib1".into()),
            quality: Quality::Hd,
            container: "mkv".into(),
            path: "/media/v1.mkv".into(),
            size_bytes: 1,
            duration_ms: 1,
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        });
        let (svc, jobs) = relink_svc(catalog);
        let target = ResolveTarget::Provider(ExternalId {
            source: "tmdb".into(),
            value: "tv/1".into(),
        });

        let outcome = svc
            .relink_series(&admin(), &SeriesId("sh1".into()), target)
            .await;

        assert!(matches!(outcome, Err(LibraryError::Unavailable(_))));
        assert!(
            jobs.list().await.unwrap().is_empty(),
            "a partial fan-out is worse than none, so nothing is queued"
        );
    }

    #[tokio::test]
    async fn relink_series_refuses_a_show_with_nothing_to_relink() {
        use domain::catalog::{Series, SeriesId};
        use domain::metadata::ExternalId;
        let catalog = MockCatalogRepo::new();
        catalog.add_series(Series {
            id: SeriesId("sh1".into()),
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
        let (svc, jobs) = relink_svc(catalog);

        let outcome = svc
            .relink_series(
                &admin(),
                &SeriesId("sh1".into()),
                ResolveTarget::Provider(ExternalId {
                    source: "tmdb".into(),
                    value: "tv/1".into(),
                }),
            )
            .await;

        assert!(
            matches!(outcome, Err(LibraryError::Unavailable(_))),
            "an empty show reports why instead of succeeding with no work"
        );
        assert!(jobs.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn relink_series_rejects_an_unknown_show_and_an_existing_target() {
        use domain::catalog::{MovieId, SeriesId, TitleId};
        use domain::metadata::ExternalId;

        let (svc, jobs) = relink_svc(MockCatalogRepo::new());
        assert!(matches!(
            svc.relink_series(
                &admin(),
                &SeriesId("ghost".into()),
                ResolveTarget::Provider(ExternalId {
                    source: "tmdb".into(),
                    value: "tv/1".into(),
                }),
            )
            .await
            .unwrap_err(),
            LibraryError::NotFound
        ));
        assert!(matches!(
            svc.relink_series(
                &admin(),
                &SeriesId("ghost".into()),
                ResolveTarget::Existing(TitleId::Movie(MovieId("m1".into()))),
            )
            .await
            .unwrap_err(),
            LibraryError::InvalidRequest(_)
        ));
        assert!(jobs.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn relink_unknown_version_is_not_found() {
        use domain::catalog::{MovieId, TitleId, VersionId};
        let (svc, _jobs) = relink_svc(MockCatalogRepo::new());
        assert!(matches!(
            svc.relink_version(
                &admin(),
                &VersionId("ghost".into()),
                ResolveTarget::Existing(TitleId::Movie(MovieId("m-new".into()))),
            )
            .await
            .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn relink_unknown_target_is_not_found() {
        use domain::catalog::{MovieId, TitleId, VersionId};
        let (svc, _jobs) = relink_svc(seeded_version_catalog());
        assert!(matches!(
            svc.relink_version(
                &admin(),
                &VersionId("v1".into()),
                ResolveTarget::Existing(TitleId::Movie(MovieId("m-missing".into()))),
            )
            .await
            .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn admin_delete_removes_the_version_and_leaves_the_title() {
        use domain::catalog::{MovieId, VersionId};
        let catalog = seeded_version_catalog();
        let (svc, jobs) = relink_svc(catalog.clone());
        let v1 = VersionId("v1".into());
        svc.delete_version(&admin(), &v1).await.unwrap();
        assert!(catalog.get_version(&v1).await.unwrap().is_none());
        assert!(
            catalog
                .get_movie(&MovieId("m-new".into()))
                .await
                .unwrap()
                .is_some()
        );
        assert!(jobs.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn delete_version_is_admin_only_and_needs_the_version_to_exist() {
        use domain::catalog::VersionId;
        let catalog = seeded_version_catalog();
        let (svc, _jobs) = relink_svc(catalog.clone());
        assert!(matches!(
            svc.delete_version(&member(), &VersionId("v1".into()))
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));
        assert!(
            catalog
                .get_version(&VersionId("v1".into()))
                .await
                .unwrap()
                .is_some()
        );
        assert!(matches!(
            svc.delete_version(&admin(), &VersionId("ghost".into()))
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    fn trigger_svc(
        catalog: MockCatalogRepo,
        transcription: bool,
        translation: bool,
        upscaling: bool,
    ) -> (Svc, MockJobStore) {
        let jobs = MockJobStore::new();
        let svc = LibraryServiceImpl::new(
            MockLibraryRepo::new(),
            MockUserRepo::new(),
            jobs.clone(),
            catalog,
            None::<MockMetadataProvider>,
        )
        .with_enrichment_flags(transcription, translation, upscaling);
        (svc, jobs)
    }

    #[tokio::test]
    async fn trigger_transcription_enqueues_for_admin_when_enabled() {
        use crate::library::TranscriptionJobPayload;
        use domain::catalog::VersionId;
        let (svc, jobs) = trigger_svc(seeded_version_catalog(), true, false, false);
        svc.trigger_transcription(&admin(), &VersionId("v1".into()), None, Some("en".into()))
            .await
            .unwrap();
        let enqueued = jobs.list().await.unwrap();
        assert_eq!(enqueued.len(), 1);
        assert_eq!(enqueued[0].kind, JobKind::Transcription);
        assert!(
            TranscriptionJobPayload::decode(&enqueued[0].payload)
                .unwrap()
                .force,
            "an admin asking for a transcription always means force"
        );
    }

    #[tokio::test]
    async fn trigger_transcription_is_forbidden_disabled_and_validated() {
        use domain::catalog::VersionId;
        let v1 = VersionId("v1".into());

        let (member_svc, _) = trigger_svc(seeded_version_catalog(), true, false, false);
        assert!(matches!(
            member_svc
                .trigger_transcription(&member(), &v1, None, None)
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));

        let (off_svc, _) = trigger_svc(seeded_version_catalog(), false, false, false);
        assert!(matches!(
            off_svc
                .trigger_transcription(&admin(), &v1, None, None)
                .await
                .unwrap_err(),
            LibraryError::Disabled
        ));

        let (missing_svc, _) = trigger_svc(MockCatalogRepo::new(), true, false, false);
        assert!(matches!(
            missing_svc
                .trigger_transcription(&admin(), &v1, None, None)
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));

        let (track_svc, _) = trigger_svc(seeded_version_catalog(), true, false, false);
        assert!(matches!(
            track_svc
                .trigger_transcription(&admin(), &v1, Some(9), None)
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn trigger_translation_enqueues_with_explicit_source() {
        use domain::catalog::VersionId;
        use domain::common::LanguageCode;
        use domain::media::{SubtitleFile, SubtitleFileId, SubtitleFormat, SubtitleSource};
        use domain::repository::CatalogRepository;

        let catalog = seeded_version_catalog();
        catalog
            .set_subtitle_files(
                &VersionId("v1".into()),
                &[SubtitleFile {
                    id: SubtitleFileId("sf1".into()),
                    version: VersionId("v1".into()),
                    language: Some(LanguageCode("en".into())),
                    format: SubtitleFormat::Srt,
                    source: SubtitleSource::External,
                    path: "/media/v1.en.srt".into(),
                    translated_from: None,
                    label: None,
                    pinned: false,
                }],
            )
            .await
            .unwrap();
        let (svc, jobs) = trigger_svc(catalog, false, true, false);
        svc.trigger_translation(
            &admin(),
            &VersionId("v1".into()),
            &SubtitleFileId("sf1".into()),
            "zh".into(),
        )
        .await
        .unwrap();
        let enqueued = jobs.list().await.unwrap();
        assert_eq!(enqueued.len(), 1);
        assert_eq!(enqueued[0].kind, JobKind::Translation);
    }

    #[tokio::test]
    async fn trigger_translation_guards_admin_enabled_and_source() {
        use domain::catalog::VersionId;
        use domain::media::SubtitleFileId;
        let v1 = VersionId("v1".into());
        let sf = SubtitleFileId("sf1".into());

        let (member_svc, _) = trigger_svc(seeded_version_catalog(), false, true, false);
        assert!(matches!(
            member_svc
                .trigger_translation(&member(), &v1, &sf, "zh".into())
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));

        let (off_svc, _) = trigger_svc(seeded_version_catalog(), false, false, false);
        assert!(matches!(
            off_svc
                .trigger_translation(&admin(), &v1, &sf, "zh".into())
                .await
                .unwrap_err(),
            LibraryError::Disabled
        ));

        let (no_source_svc, _) = trigger_svc(seeded_version_catalog(), false, true, false);
        assert!(matches!(
            no_source_svc
                .trigger_translation(&admin(), &v1, &sf, "zh".into())
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn trigger_upscale_enqueues_and_guards() {
        use domain::catalog::VersionId;
        let v1 = VersionId("v1".into());

        let (svc, jobs) = trigger_svc(seeded_version_catalog(), false, false, true);
        svc.trigger_upscale(&admin(), &v1, 2160).await.unwrap();
        let enqueued = jobs.list().await.unwrap();
        assert_eq!(enqueued.len(), 1);
        assert_eq!(enqueued[0].kind, JobKind::Upscale);

        let (member_svc, _) = trigger_svc(seeded_version_catalog(), false, false, true);
        assert!(matches!(
            member_svc
                .trigger_upscale(&member(), &v1, 2160)
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));

        let (off_svc, _) = trigger_svc(seeded_version_catalog(), false, false, false);
        assert!(matches!(
            off_svc
                .trigger_upscale(&admin(), &v1, 2160)
                .await
                .unwrap_err(),
            LibraryError::Disabled
        ));

        let (missing_svc, _) = trigger_svc(MockCatalogRepo::new(), false, false, true);
        assert!(matches!(
            missing_svc
                .trigger_upscale(&admin(), &v1, 2160)
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn trigger_combine_enqueues_and_guards() {
        use domain::catalog::VersionId;
        use domain::common::LanguageCode;
        use domain::media::{SubtitleFile, SubtitleFileId, SubtitleFormat, SubtitleSource};
        use domain::repository::CatalogRepository;

        let v1 = VersionId("v1".into());
        let en = SubtitleFileId("sf-en".into());
        let fr = SubtitleFileId("sf-fr".into());
        let seed = || {
            let catalog = seeded_version_catalog();
            let files = vec![
                SubtitleFile {
                    id: en.clone(),
                    version: v1.clone(),
                    language: Some(LanguageCode("en".into())),
                    format: SubtitleFormat::Srt,
                    source: SubtitleSource::External,
                    path: "/media/v1.en.srt".into(),
                    translated_from: None,
                    label: None,
                    pinned: false,
                },
                SubtitleFile {
                    id: fr.clone(),
                    version: v1.clone(),
                    language: Some(LanguageCode("fr".into())),
                    format: SubtitleFormat::Srt,
                    source: SubtitleSource::External,
                    path: "/media/v1.fr.srt".into(),
                    translated_from: None,
                    label: None,
                    pinned: false,
                },
            ];
            (catalog, files)
        };

        let (catalog, files) = seed();
        catalog.set_subtitle_files(&v1, &files).await.unwrap();
        let (svc, jobs) = trigger_svc(catalog, false, false, false);
        svc.trigger_combine(&admin(), &v1, &en, &fr).await.unwrap();
        let enqueued = jobs.list().await.unwrap();
        assert_eq!(enqueued.len(), 1);
        assert_eq!(enqueued[0].kind, JobKind::Combine);

        let (catalog, files) = seed();
        catalog.set_subtitle_files(&v1, &files).await.unwrap();
        let (member_svc, _) = trigger_svc(catalog, false, false, false);
        assert!(matches!(
            member_svc
                .trigger_combine(&member(), &v1, &en, &fr)
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));

        let (catalog, files) = seed();
        catalog.set_subtitle_files(&v1, &files).await.unwrap();
        let (source_svc, _) = trigger_svc(catalog, false, false, false);
        assert!(matches!(
            source_svc
                .trigger_combine(&admin(), &v1, &en, &SubtitleFileId("absent".into()))
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));

        let (missing_svc, _) = trigger_svc(MockCatalogRepo::new(), false, false, false);
        assert!(matches!(
            missing_svc
                .trigger_combine(&admin(), &v1, &en, &fr)
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    fn edit_svc() -> (Svc, MockCatalogRepo) {
        let catalog = refresh_catalog();
        let libraries = MockLibraryRepo::new();
        let mut films = library("films");
        films.sort_articles = vec!["the".into(), "a".into()];
        libraries.insert_library(films);
        let mut shows = library("shows");
        shows.kind = LibraryKind::Tv;
        shows.sort_articles = vec!["the".into()];
        libraries.insert_library(shows);
        let svc = LibraryServiceImpl::new(
            libraries,
            MockUserRepo::new(),
            MockJobStore::new(),
            catalog.clone(),
            None::<MockMetadataProvider>,
        );
        (svc, catalog)
    }

    fn movie_edit(title: &str) -> MovieEdit {
        MovieEdit {
            title: title.into(),
            year: Some(1999),
            overview: Some("hand written".into()),
            runtime_minutes: Some(136),
            content_rating: None,
        }
    }

    #[tokio::test]
    async fn editing_a_movie_flags_it_and_derives_the_sort_title() {
        let (svc, catalog) = edit_svc();
        let id = MovieId("m1".into());

        let edited = svc
            .edit_movie(&admin(), &id, movie_edit("The Matrix"))
            .await
            .unwrap();

        assert_eq!(edited.title, "The Matrix");
        assert_eq!(
            edited.sort_title, "matrix, the",
            "sort_title comes from the owning library's articles, never from the caller"
        );
        assert_eq!(edited.year, Some(1999));
        assert_eq!(edited.runtime_minutes, Some(136));
        assert!(edited.manually_edited);

        let stored = catalog.get_movie(&id).await.unwrap().unwrap();
        assert_eq!(stored.sort_title, "matrix, the");
        assert!(stored.manually_edited);
        assert_eq!(
            stored.added_at,
            Timestamp::UNIX_EPOCH,
            "an edit must not reset added_at"
        );
    }

    #[tokio::test]
    async fn editing_a_movie_clears_the_fields_the_request_omits() {
        let (svc, _) = edit_svc();
        let edited = svc
            .edit_movie(
                &admin(),
                &MovieId("m1".into()),
                MovieEdit {
                    title: "Bare".into(),
                    year: None,
                    overview: None,
                    runtime_minutes: None,
                    content_rating: None,
                },
            )
            .await
            .unwrap();
        assert!(edited.year.is_none());
        assert!(edited.overview.is_none());
        assert!(edited.runtime_minutes.is_none());
        assert!(edited.content_rating.is_none());
    }

    #[tokio::test]
    async fn editing_a_series_flags_it_and_derives_the_sort_title() {
        let (svc, catalog) = edit_svc();
        let id = SeriesId("s1".into());

        let edited = svc
            .edit_series(
                &admin(),
                &id,
                SeriesEdit {
                    title: "The Wire".into(),
                    year: Some(2002),
                    overview: None,
                    content_rating: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(edited.sort_title, "wire, the");
        assert!(edited.manually_edited);
        assert!(
            catalog
                .get_series(&id)
                .await
                .unwrap()
                .unwrap()
                .manually_edited
        );
    }

    #[tokio::test]
    async fn editing_an_episode_flags_it_and_keeps_its_season_and_number() {
        let (svc, catalog) = edit_svc();
        let id = EpisodeId("e1".into());
        let before = catalog.get_episode(&id).await.unwrap().unwrap();

        let edited = svc
            .edit_episode(
                &admin(),
                &id,
                EpisodeEdit {
                    title: "Pilot".into(),
                    overview: Some("it begins".into()),
                    runtime_minutes: Some(42),
                    air_date: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(edited.title, "Pilot");
        assert_eq!(edited.season, before.season);
        assert_eq!(edited.number, before.number);
        assert!(edited.manually_edited);
    }

    #[tokio::test]
    async fn editing_is_admin_only() {
        let (svc, _) = edit_svc();
        assert!(matches!(
            svc.edit_movie(&member(), &MovieId("m1".into()), movie_edit("Nope"))
                .await
                .unwrap_err(),
            LibraryError::Forbidden
        ));
        assert!(matches!(
            svc.edit_series(
                &member(),
                &SeriesId("s1".into()),
                SeriesEdit {
                    title: "Nope".into(),
                    year: None,
                    overview: None,
                    content_rating: None,
                }
            )
            .await
            .unwrap_err(),
            LibraryError::Forbidden
        ));
        assert!(matches!(
            svc.edit_episode(
                &member(),
                &EpisodeId("e1".into()),
                EpisodeEdit {
                    title: "Nope".into(),
                    overview: None,
                    runtime_minutes: None,
                    air_date: None,
                }
            )
            .await
            .unwrap_err(),
            LibraryError::Forbidden
        ));
    }

    #[tokio::test]
    async fn editing_a_missing_row_is_not_found() {
        let (svc, _) = edit_svc();
        assert!(matches!(
            svc.edit_movie(&admin(), &MovieId("ghost".into()), movie_edit("Ghost"))
                .await
                .unwrap_err(),
            LibraryError::NotFound
        ));
        assert!(matches!(
            svc.edit_series(
                &admin(),
                &SeriesId("ghost".into()),
                SeriesEdit {
                    title: "Ghost".into(),
                    year: None,
                    overview: None,
                    content_rating: None,
                }
            )
            .await
            .unwrap_err(),
            LibraryError::NotFound
        ));
        assert!(matches!(
            svc.edit_episode(
                &admin(),
                &EpisodeId("ghost".into()),
                EpisodeEdit {
                    title: "Ghost".into(),
                    overview: None,
                    runtime_minutes: None,
                    air_date: None,
                }
            )
            .await
            .unwrap_err(),
            LibraryError::NotFound
        ));
    }

    #[tokio::test]
    async fn a_force_refresh_is_carried_on_the_queued_job() {
        let (svc, jobs) = refresh_svc(refresh_catalog());
        svc.reidentify(&admin(), TitleRef::Movie(MovieId("m1".into())), None, true)
            .await
            .unwrap();
        let enqueued = jobs.list().await.unwrap();
        assert!(matches!(
            MetadataJobPayload::decode(&enqueued[0].payload).unwrap(),
            MetadataJobPayload::Title { force: true, .. }
        ));
    }

    #[tokio::test]
    async fn a_whole_library_refresh_is_never_forced() {
        let (svc, jobs) = refresh_svc(refresh_catalog());
        svc.refresh_library_metadata(&admin(), &LibraryId("films".into()))
            .await
            .unwrap();
        let enqueued = jobs.list().await.unwrap();
        assert!(!enqueued.is_empty());
        for job in &enqueued {
            assert!(
                matches!(
                    MetadataJobPayload::decode(&job.payload).unwrap(),
                    MetadataJobPayload::Title { force: false, .. }
                ),
                "a bulk refresh must never discard manual edits"
            );
        }
    }
}

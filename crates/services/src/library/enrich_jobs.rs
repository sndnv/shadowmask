use domain::catalog::{ArtworkId, ArtworkOwner, VersionId};
use domain::error::RepositoryError;
use domain::job::{JobId, JobKind, JobPriority};
use domain::library::DiscoveredFile;
use domain::media::AudioTrack;
use domain::metadata::{Artwork, ArtworkKind, PersonId};
use domain::repository::JobRepository;
use jiff::Timestamp;
use uuid::Uuid;

use super::{
    ArtworkJobItem, ArtworkJobPayload, MetadataJobPayload, SubtitleJobPayload,
    TranscriptionJobPayload, TrickplayJobPayload, select_audio_track, translation_job,
};
use crate::job::queued_job;

pub(super) struct SubtitleContext {
    pub(super) imdb_id: Option<String>,
    pub(super) title: String,
    pub(super) season: Option<u16>,
    pub(super) episode: Option<u16>,
}

pub(super) struct EnrichmentJobs<J> {
    jobs: J,
    pub(super) subtitle_languages: Vec<String>,
    pub(super) transcription_enabled: bool,
    pub(super) translation_languages: Vec<String>,
}

impl<J> EnrichmentJobs<J> {
    pub(super) fn new(jobs: J) -> Self {
        Self {
            jobs,
            subtitle_languages: Vec::new(),
            transcription_enabled: false,
            translation_languages: Vec::new(),
        }
    }
}

impl<J> EnrichmentJobs<J>
where
    J: JobRepository + Send + Sync,
{
    pub(super) async fn for_file(
        &self,
        version_id: &VersionId,
        file: &DiscoveredFile,
        subtitle: &SubtitleContext,
        has_native_subtitle: bool,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        let audio = &file.probe.audio;
        self.enqueue_trickplay(version_id, &file.path, file.probe.duration_ms, parent)
            .await?;
        let transcription_wanted =
            self.transcription_enabled && !has_native_subtitle && !audio.is_empty();
        let opensubtitles_configured = !self.subtitle_languages.is_empty();
        self.enqueue_subtitles(
            version_id,
            subtitle,
            transcription_wanted && opensubtitles_configured,
            parent,
        )
        .await?;
        if !opensubtitles_configured {
            self.enqueue_transcription(version_id, &file.path, has_native_subtitle, audio, parent)
                .await?;
        }
        self.enqueue_translation(version_id, has_native_subtitle, parent)
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
            force: false,
        }
        .encode();
        let now = Timestamp::now();
        self.jobs
            .enqueue(queued_job(
                JobKind::Transcription,
                JobPriority::Low,
                raw,
                parent,
                now,
            ))
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
        .encode();
        let now = Timestamp::now();
        self.jobs
            .enqueue(queued_job(
                JobKind::Subtitles,
                JobPriority::Normal,
                raw,
                parent,
                now,
            ))
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
        .encode();
        let now = Timestamp::now();
        self.jobs
            .enqueue(queued_job(
                JobKind::Trickplay,
                JobPriority::Normal,
                raw,
                parent,
                now,
            ))
            .await
    }

    pub(super) async fn enqueue_person_metadata(
        &self,
        ids: Vec<PersonId>,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        if ids.is_empty() {
            return Ok(());
        }
        let raw = MetadataJobPayload::People { ids, force: false }.encode();
        let now = Timestamp::now();
        self.jobs
            .enqueue(queued_job(
                JobKind::Metadata,
                JobPriority::Low,
                raw,
                parent,
                now,
            ))
            .await
    }

    pub(super) async fn enqueue_artwork(
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
        let raw = ArtworkJobPayload { owner, items }.encode();
        let now = Timestamp::now();
        self.jobs
            .enqueue(queued_job(
                JobKind::Artwork,
                JobPriority::Normal,
                raw,
                parent,
                now,
            ))
            .await
    }
}

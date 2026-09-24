use domain::catalog::VersionId;
use domain::error::RepositoryError;
use domain::job::{JobId, TranscriptionTrigger};
use domain::repository::{CatalogRepository, JobRepository};

use super::{select_audio_track, transcription_job};

pub struct TranscriptionEnqueuer<J, C> {
    jobs: J,
    catalog: C,
    subtitle_languages: Vec<String>,
    enabled: bool,
}

impl<J, C> TranscriptionEnqueuer<J, C> {
    pub fn new(jobs: J, catalog: C, subtitle_languages: Vec<String>, enabled: bool) -> Self {
        Self { jobs, catalog, subtitle_languages, enabled }
    }
}

impl<J, C> TranscriptionTrigger for TranscriptionEnqueuer<J, C>
where
    J: JobRepository + Send + Sync,
    C: CatalogRepository + Send + Sync,
{
    async fn trigger(
        &self,
        version_id: &VersionId,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        if !self.enabled {
            return Ok(());
        }
        let Some(detail) = self.catalog.version_detail(version_id).await? else {
            return Ok(());
        };
        if !detail.subtitle_files.is_empty() || detail.audio.is_empty() {
            return Ok(());
        }
        let preferred = self.subtitle_languages.first().map(String::as_str);
        let audio_track_index = select_audio_track(&detail.audio, preferred);
        let source_language = audio_track_index
            .and_then(|index| detail.audio.iter().find(|track| track.index == index))
            .or_else(|| detail.audio.first())
            .and_then(|track| track.language.as_ref())
            .map(|code| code.0.clone());
        let mut job = transcription_job(
            version_id,
            &detail.version.path,
            source_language,
            audio_track_index,
            false,
        );
        job.parent_id = parent.cloned();
        self.jobs.enqueue(job).await
    }
}

#[cfg(test)]
mod tests {
    use domain::catalog::{TitleId, Version};
    use domain::common::{LanguageCode, Quality};
    use domain::job::{JobId, JobKind};
    use domain::media::{AudioTrack, SubtitleFile, SubtitleFileId, SubtitleFormat, SubtitleSource};
    use jiff::Timestamp;

    use super::*;
    use crate::library::TranscriptionJobPayload;
    use mocks::{MockCatalogRepo, MockJobStore};

    fn track(index: u32, language: Option<&str>) -> AudioTrack {
        AudioTrack {
            index,
            codec: "aac".into(),
            channels: 2,
            language: language.map(|code| LanguageCode(code.into())),
            bitrate: None,
        }
    }

    fn version() -> Version {
        Version {
            id: VersionId("v1".into()),
            title: TitleId::Movie(domain::catalog::MovieId("m1".into())),
            library: domain::library::LibraryId("lib".into()),
            quality: Quality::Hd,
            container: "mkv".into(),
            path: "/m/v1.mkv".into(),
            size_bytes: 1,
            duration_ms: 1000,
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    async fn seed(catalog: &MockCatalogRepo, audio: Vec<AudioTrack>) {
        catalog.add_version(version());
        catalog.set_version_tracks(&VersionId("v1".into()), &[], &audio, &[], &[]).await.unwrap();
    }

    #[tokio::test]
    async fn enqueues_transcription_selecting_preferred_audio_track() {
        let catalog = MockCatalogRepo::new();
        seed(&catalog, vec![track(1, Some("eng")), track(2, Some("spa"))]).await;
        let jobs = MockJobStore::new();
        let enqueuer =
            TranscriptionEnqueuer::new(jobs.clone(), catalog.clone(), vec!["spa".into()], true);

        enqueuer.trigger(&VersionId("v1".into()), None).await.unwrap();

        let job = jobs
            .list()
            .await
            .unwrap()
            .into_iter()
            .find(|j| j.kind == JobKind::Transcription)
            .unwrap();
        let payload = TranscriptionJobPayload::decode(&job.payload).unwrap();
        assert_eq!(payload.audio_track_index, Some(2));
        assert_eq!(payload.source_language.as_deref(), Some("spa"));
        assert_eq!(payload.source_path, "/m/v1.mkv");
    }

    #[tokio::test]
    async fn links_the_child_to_its_parent_job() {
        let catalog = MockCatalogRepo::new();
        seed(&catalog, vec![track(1, Some("eng"))]).await;
        let jobs = MockJobStore::new();
        let enqueuer = TranscriptionEnqueuer::new(jobs.clone(), catalog, Vec::new(), true);

        enqueuer.trigger(&VersionId("v1".into()), Some(&JobId("parent".into()))).await.unwrap();

        let job = jobs
            .list()
            .await
            .unwrap()
            .into_iter()
            .find(|j| j.kind == JobKind::Transcription)
            .unwrap();
        assert_eq!(job.parent_id, Some(JobId("parent".into())));
    }

    #[tokio::test]
    async fn labels_from_first_audio_track_without_a_preference() {
        let catalog = MockCatalogRepo::new();
        seed(&catalog, vec![track(1, Some("eng"))]).await;
        let jobs = MockJobStore::new();
        let enqueuer = TranscriptionEnqueuer::new(jobs.clone(), catalog.clone(), Vec::new(), true);

        enqueuer.trigger(&VersionId("v1".into()), None).await.unwrap();

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
    async fn no_op_when_version_missing() {
        let jobs = MockJobStore::new();
        let enqueuer =
            TranscriptionEnqueuer::new(jobs.clone(), MockCatalogRepo::new(), Vec::new(), true);

        enqueuer.trigger(&VersionId("gone".into()), None).await.unwrap();

        assert!(jobs.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn no_op_without_audio() {
        let catalog = MockCatalogRepo::new();
        seed(&catalog, Vec::new()).await;
        let jobs = MockJobStore::new();
        let enqueuer = TranscriptionEnqueuer::new(jobs.clone(), catalog, Vec::new(), true);

        enqueuer.trigger(&VersionId("v1".into()), None).await.unwrap();

        assert!(jobs.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn no_op_when_subtitles_already_present() {
        let catalog = MockCatalogRepo::new();
        seed(&catalog, vec![track(1, Some("eng"))]).await;
        catalog
            .set_subtitle_files(
                &VersionId("v1".into()),
                &[SubtitleFile {
                    id: SubtitleFileId("os".into()),
                    version: VersionId("v1".into()),
                    language: Some(LanguageCode("en".into())),
                    format: SubtitleFormat::Srt,
                    source: SubtitleSource::OpenSubtitles,
                    path: "/subs/os.srt".into(),
                    translated_from: None,
                    label: None,
                    pinned: false,
                }],
            )
            .await
            .unwrap();
        let jobs = MockJobStore::new();
        let enqueuer = TranscriptionEnqueuer::new(jobs.clone(), catalog, Vec::new(), true);

        enqueuer.trigger(&VersionId("v1".into()), None).await.unwrap();

        assert!(jobs.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn nothing_is_enqueued_when_transcription_is_switched_off() {
        let catalog = MockCatalogRepo::new();
        seed(&catalog, vec![track(1, Some("eng"))]).await;
        let jobs = MockJobStore::new();
        let enqueuer = TranscriptionEnqueuer::new(jobs.clone(), catalog, Vec::new(), false);

        enqueuer.trigger(&VersionId("v1".into()), None).await.unwrap();

        assert!(jobs.list().await.unwrap().is_empty());
    }
}

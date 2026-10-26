use domain::catalog::{ArtworkOwner, LibraryFootprint, TitleRef, VersionId};
use domain::job::{Job, JobKind};
use domain::library::LibraryId;

use super::{
    ArtworkJobPayload, CombineJobPayload, FetchJobPayload, IngestJobPayload, MetadataJobPayload,
    PlaybackPrepJobPayload, RelinkJobPayload, ScanJobPayload, SubtitleJobPayload,
    TranscriptionJobPayload, TranslationJobPayload, TrickplayJobPayload, UpscaleJobPayload,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum JobScope {
    Library(LibraryId),
    Version(VersionId),
    Owner(ArtworkOwner),
    Title(TitleRef),
    Server,
    Elsewhere,
    Unreadable,
}

impl JobScope {
    pub(super) fn of(job: &Job) -> Self {
        let raw = job.payload.as_str();
        match job.kind {
            JobKind::LibraryScan => JobScope::Library(ScanJobPayload::decode(raw).library),
            JobKind::Ingest => IngestJobPayload::decode(raw)
                .map_or(JobScope::Unreadable, |payload| JobScope::Library(payload.library)),
            JobKind::Relink => RelinkJobPayload::decode(raw)
                .map_or(JobScope::Unreadable, |payload| JobScope::Library(payload.library)),
            JobKind::Fetch => FetchJobPayload::decode(raw)
                .map_or(JobScope::Unreadable, |payload| JobScope::Library(payload.library)),
            JobKind::Subtitles => SubtitleJobPayload::decode(raw)
                .map_or(JobScope::Unreadable, |payload| JobScope::Version(payload.version_id)),
            JobKind::Translation => TranslationJobPayload::decode(raw)
                .map_or(JobScope::Unreadable, |payload| JobScope::Version(payload.version_id)),
            JobKind::Combine => CombineJobPayload::decode(raw)
                .map_or(JobScope::Unreadable, |payload| JobScope::Version(payload.version_id)),
            JobKind::Upscale => UpscaleJobPayload::decode(raw)
                .map_or(JobScope::Unreadable, |payload| JobScope::Version(payload.version_id)),
            JobKind::Trickplay => TrickplayJobPayload::decode(raw)
                .map_or(JobScope::Unreadable, |payload| JobScope::Version(payload.version_id)),
            JobKind::PlaybackPrep => PlaybackPrepJobPayload::decode(raw)
                .map_or(JobScope::Unreadable, |payload| JobScope::Version(payload.version_id)),
            JobKind::Transcription => TranscriptionJobPayload::decode(raw)
                .map_or(JobScope::Unreadable, |payload| JobScope::Version(payload.version_id)),
            JobKind::Metadata => match MetadataJobPayload::decode(raw) {
                Ok(MetadataJobPayload::Title { title, .. }) => JobScope::Title(title),
                Ok(MetadataJobPayload::People { .. }) => JobScope::Elsewhere,
                Err(_) => JobScope::Unreadable,
            },
            JobKind::Artwork => ArtworkJobPayload::decode(raw)
                .map_or(JobScope::Unreadable, |payload| JobScope::Owner(payload.owner)),
            JobKind::SearchReindex | JobKind::Dedup => JobScope::Server,
            JobKind::Fingerprint
            | JobKind::ScheduledScan
            | JobKind::Retention
            | JobKind::OrphanSweep
            | JobKind::CacheEviction => JobScope::Elsewhere,
        }
    }

    pub(super) fn is_of(&self, library: &LibraryId, footprint: &LibraryFootprint) -> bool {
        match self {
            JobScope::Library(id) => id == library,
            JobScope::Version(id) => footprint.versions.contains(id),
            JobScope::Owner(owner) => footprint.titles.holds(owner),
            JobScope::Title(title) => footprint.titles.holds_title(title),
            JobScope::Server | JobScope::Elsewhere | JobScope::Unreadable => false,
        }
    }

    pub(super) fn blocks_purge(&self, library: &LibraryId, footprint: &LibraryFootprint) -> bool {
        matches!(self, JobScope::Server | JobScope::Unreadable) || self.is_of(library, footprint)
    }

    pub(super) fn blocks_full_clean(&self) -> bool {
        match self {
            JobScope::Owner(owner) => {
                !matches!(owner, ArtworkOwner::Collection(_) | ArtworkOwner::Person(_))
            }
            JobScope::Library(_) | JobScope::Title(_) | JobScope::Server | JobScope::Unreadable => {
                true
            }
            JobScope::Version(_) | JobScope::Elsewhere => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use domain::catalog::{
        CollectionId, EpisodeId, MovieId, SeasonId, SeriesId, TitleId, TitleSet,
    };
    use domain::job::JobPriority;
    use domain::library::{LibraryKind, ResolveTarget, ScanMode, UnmatchedFileId};
    use domain::metadata::PersonId;
    use jiff::Timestamp;

    use super::*;
    use crate::job::queued_job;

    fn job(kind: JobKind, payload: String) -> Job {
        queued_job(kind, JobPriority::Normal, payload, None, Timestamp::UNIX_EPOCH)
    }

    fn lib() -> LibraryId {
        LibraryId("lib".into())
    }

    fn version() -> VersionId {
        VersionId("v".into())
    }

    fn target() -> ResolveTarget {
        ResolveTarget::Existing(TitleId::Movie(MovieId("m".into())))
    }

    fn readable() -> Vec<(Job, JobScope)> {
        let on_version = JobScope::Version(version());
        let movie = TitleRef::Movie(MovieId("m".into()));
        let season = ArtworkOwner::Season(SeasonId("se".into()));
        vec![
            (
                job(
                    JobKind::LibraryScan,
                    ScanJobPayload { library: lib(), mode: ScanMode::Reread }.encode(),
                ),
                JobScope::Library(lib()),
            ),
            (
                job(
                    JobKind::Ingest,
                    IngestJobPayload {
                        library: lib(),
                        unmatched: UnmatchedFileId("u".into()),
                        path: "/m/a.mkv".into(),
                        target: target(),
                    }
                    .encode(),
                ),
                JobScope::Library(lib()),
            ),
            (
                job(
                    JobKind::Relink,
                    RelinkJobPayload {
                        version: version(),
                        library: lib(),
                        path: "/m/a.mkv".into(),
                        target: target(),
                    }
                    .encode(),
                ),
                JobScope::Library(lib()),
            ),
            (
                job(
                    JobKind::Fetch,
                    FetchJobPayload {
                        library: lib(),
                        source_url: "https://example.org/a".into(),
                        kind: LibraryKind::Movie,
                        title: "A".into(),
                        year: None,
                        external_id: None,
                        season: None,
                        episode: None,
                    }
                    .encode(),
                ),
                JobScope::Library(lib()),
            ),
            (
                job(
                    JobKind::Subtitles,
                    SubtitleJobPayload {
                        version_id: version(),
                        imdb_id: None,
                        title: None,
                        languages: vec!["en".into()],
                        season: None,
                        episode: None,
                        transcribe_on_miss: false,
                    }
                    .encode(),
                ),
                on_version.clone(),
            ),
            (
                job(
                    JobKind::Translation,
                    TranslationJobPayload {
                        version_id: version(),
                        target_languages: vec!["fr".into()],
                        source_subtitle_id: None,
                    }
                    .encode(),
                ),
                on_version.clone(),
            ),
            (
                job(
                    JobKind::Combine,
                    CombineJobPayload {
                        version_id: version(),
                        top_subtitle_id: "a".into(),
                        bottom_subtitle_id: "b".into(),
                    }
                    .encode(),
                ),
                on_version.clone(),
            ),
            (
                job(
                    JobKind::Upscale,
                    UpscaleJobPayload { version_id: version(), target_height: 2160 }.encode(),
                ),
                on_version.clone(),
            ),
            (
                job(
                    JobKind::Trickplay,
                    TrickplayJobPayload {
                        version_id: version(),
                        source_path: "/m/a.mkv".into(),
                        duration_ms: 1,
                    }
                    .encode(),
                ),
                on_version.clone(),
            ),
            (
                job(
                    JobKind::PlaybackPrep,
                    PlaybackPrepJobPayload { version_id: version() }.encode(),
                ),
                on_version.clone(),
            ),
            (
                job(
                    JobKind::Transcription,
                    TranscriptionJobPayload {
                        version_id: version(),
                        source_path: "/m/a.mkv".into(),
                        source_language: None,
                        language_hint: None,
                        audio_track_index: None,
                        force: false,
                    }
                    .encode(),
                ),
                on_version,
            ),
            (
                job(
                    JobKind::Metadata,
                    MetadataJobPayload::Title {
                        title: movie.clone(),
                        external_id: None,
                        force: false,
                    }
                    .encode(),
                ),
                JobScope::Title(movie),
            ),
            (
                job(
                    JobKind::Metadata,
                    MetadataJobPayload::People { ids: vec![PersonId("p".into())], force: false }
                        .encode(),
                ),
                JobScope::Elsewhere,
            ),
            (
                job(
                    JobKind::Artwork,
                    ArtworkJobPayload { owner: season.clone(), items: Vec::new() }.encode(),
                ),
                JobScope::Owner(season),
            ),
            (job(JobKind::SearchReindex, String::new()), JobScope::Server),
            (job(JobKind::Dedup, String::new()), JobScope::Server),
            (job(JobKind::Fingerprint, String::new()), JobScope::Elsewhere),
            (job(JobKind::ScheduledScan, String::new()), JobScope::Elsewhere),
            (job(JobKind::Retention, String::new()), JobScope::Elsewhere),
            (job(JobKind::OrphanSweep, String::new()), JobScope::Elsewhere),
            (job(JobKind::CacheEviction, String::new()), JobScope::Elsewhere),
        ]
    }

    #[test]
    fn every_kind_is_traced_to_what_its_payload_names() {
        let cases = readable();
        let kinds: HashSet<JobKind> = cases.iter().map(|(job, _)| job.kind).collect();
        assert_eq!(kinds.len(), JobKind::ALL.len(), "every kind has a case");
        for (job, scope) in cases {
            assert_eq!(JobScope::of(&job), scope, "{:?}", job.kind);
        }
    }

    #[test]
    fn a_payload_that_cannot_be_read_is_unreadable() {
        for kind in [
            JobKind::Ingest,
            JobKind::Relink,
            JobKind::Fetch,
            JobKind::Subtitles,
            JobKind::Translation,
            JobKind::Combine,
            JobKind::Upscale,
            JobKind::Trickplay,
            JobKind::PlaybackPrep,
            JobKind::Transcription,
            JobKind::Metadata,
            JobKind::Artwork,
        ] {
            assert_eq!(JobScope::of(&job(kind, "{".into())), JobScope::Unreadable, "{kind:?}");
        }
    }

    #[test]
    fn a_job_is_the_librarys_when_its_library_version_or_title_is() {
        let footprint = LibraryFootprint {
            versions: HashSet::from([version()]),
            titles: TitleSet {
                movies: HashSet::from([MovieId("m".into())]),
                series: HashSet::from([SeriesId("s".into())]),
                seasons: HashSet::new(),
                episodes: HashSet::from([EpisodeId("e".into())]),
            },
        };
        let ours = [
            JobScope::Library(lib()),
            JobScope::Version(version()),
            JobScope::Owner(ArtworkOwner::Episode(EpisodeId("e".into()))),
            JobScope::Title(TitleRef::Series(SeriesId("s".into()))),
        ];
        for scope in ours {
            assert!(scope.is_of(&lib(), &footprint), "{scope:?}");
            assert!(scope.blocks_purge(&lib(), &footprint), "{scope:?}");
        }
        let theirs = [
            JobScope::Library(LibraryId("other".into())),
            JobScope::Version(VersionId("other".into())),
            JobScope::Owner(ArtworkOwner::Collection(CollectionId("c".into()))),
            JobScope::Title(TitleRef::Movie(MovieId("other".into()))),
            JobScope::Elsewhere,
        ];
        for scope in theirs {
            assert!(!scope.is_of(&lib(), &footprint), "{scope:?}");
            assert!(!scope.blocks_purge(&lib(), &footprint), "{scope:?}");
        }
        for scope in [JobScope::Server, JobScope::Unreadable] {
            assert!(!scope.is_of(&lib(), &footprint), "{scope:?} is nobody's to cancel");
            assert!(scope.blocks_purge(&lib(), &footprint), "{scope:?} may touch any library");
        }
    }

    #[test]
    fn a_full_clean_waits_for_work_on_any_library_or_title_but_not_on_a_file() {
        for scope in [
            JobScope::Library(LibraryId("other".into())),
            JobScope::Owner(ArtworkOwner::Season(SeasonId("se".into()))),
            JobScope::Title(TitleRef::Movie(MovieId("other".into()))),
            JobScope::Server,
            JobScope::Unreadable,
        ] {
            assert!(scope.blocks_full_clean(), "{scope:?}");
        }
        for scope in [
            JobScope::Version(version()),
            JobScope::Owner(ArtworkOwner::Collection(CollectionId("c".into()))),
            JobScope::Owner(ArtworkOwner::Person(PersonId("p".into()))),
            JobScope::Elsewhere,
        ] {
            assert!(!scope.blocks_full_clean(), "{scope:?}");
        }
    }
}

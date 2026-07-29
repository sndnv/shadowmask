use serde::Serialize;

use domain::catalog::VersionDetail;
use domain::media::{
    AudioTrack, Chapter, DetectedMarkers, EmbeddedSubtitleTrack, HdrFormat, SubtitleFile,
    SubtitleFormat, SubtitleSource, TrickplayAsset, VideoTrack,
};

use crate::dto::common::{QualityDto, TitleRefDto};

#[derive(Debug, Serialize)]
pub struct VersionDetailResponse {
    pub id: String,
    pub title: TitleRefDto,
    pub library_id: String,
    pub quality: QualityDto,
    pub container: String,
    pub size_bytes: u64,
    pub duration_ms: u64,
    pub edition: Option<String>,
    pub available: bool,
    pub added_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub video: Vec<VideoTrackDto>,
    pub audio: Vec<AudioTrackDto>,
    pub subtitles: Vec<SubtitleTrackDto>,
    pub subtitle_files: Vec<SubtitleFileDto>,
    pub chapters: Vec<ChapterDto>,
    pub markers: MarkersDto,
    pub trickplay: Vec<TrickplayRefDto>,
}

#[derive(Debug, Serialize)]
pub struct VideoTrackDto {
    pub index: u32,
    pub codec: String,
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,
    pub hdr: Option<HdrFormatDto>,
    pub frame_rate: f32,
    pub bitrate: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HdrFormatDto {
    Hdr10,
    Hdr10Plus,
    DolbyVision,
    Hlg,
}

#[derive(Debug, Serialize)]
pub struct AudioTrackDto {
    pub index: u32,
    pub codec: String,
    pub channels: u8,
    pub language: Option<String>,
    pub bitrate: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct SubtitleTrackDto {
    pub index: u32,
    pub language: Option<String>,
    pub format: SubtitleFormatDto,
    pub forced: bool,
    pub default: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubtitleFormatDto {
    Srt,
    Ass,
    Vtt,
    Pgs,
    VobSub,
}

#[derive(Debug, Serialize)]
pub struct SubtitleFileDto {
    pub id: String,
    pub language: Option<String>,
    pub format: SubtitleFormatDto,
    pub source: SubtitleSourceDto,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubtitleSourceDto {
    OpenSubtitles,
    External,
    Generated,
    MachineTranslated,
    Combined,
}

#[derive(Debug, Serialize)]
pub struct ChapterDto {
    pub title: String,
    pub start_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct MarkersDto {
    pub intro: Vec<MarkerDto>,
    pub credits: Vec<MarkerDto>,
}

#[derive(Debug, Serialize)]
pub struct MarkerDto {
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct TrickplayRefDto {
    pub interval_ms: u64,
    pub columns: u32,
    pub rows: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub sheets: usize,
}

impl VersionDetailResponse {
    pub fn with_path(detail: VersionDetail, include_path: bool) -> Self {
        let path = include_path.then(|| detail.version.path.clone());
        let mut response = VersionDetailResponse::from(detail);
        response.path = path;
        response
    }
}

impl From<VersionDetail> for VersionDetailResponse {
    fn from(d: VersionDetail) -> Self {
        VersionDetailResponse {
            id: d.version.id.0,
            title: d.version.title.into(),
            library_id: d.version.library.0,
            quality: d.version.quality.into(),
            container: d.version.container,
            size_bytes: d.version.size_bytes,
            duration_ms: d.version.duration_ms,
            edition: d.version.edition,
            available: d.version.available,
            added_at: d.version.added_at.to_string(),
            updated_at: d.version.updated_at.to_string(),
            path: None,
            video: d.video.into_iter().map(Into::into).collect(),
            audio: d.audio.into_iter().map(Into::into).collect(),
            subtitles: d.subtitles.into_iter().map(Into::into).collect(),
            subtitle_files: d.subtitle_files.into_iter().map(Into::into).collect(),
            chapters: d.chapters.into_iter().map(Into::into).collect(),
            markers: d.markers.into(),
            trickplay: d.trickplay.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<VideoTrack> for VideoTrackDto {
    fn from(t: VideoTrack) -> Self {
        VideoTrackDto {
            index: t.index,
            codec: t.codec,
            width: t.width,
            height: t.height,
            bit_depth: t.bit_depth,
            hdr: t.hdr.map(Into::into),
            frame_rate: t.frame_rate,
            bitrate: t.bitrate,
        }
    }
}

impl From<HdrFormat> for HdrFormatDto {
    fn from(h: HdrFormat) -> Self {
        match h {
            HdrFormat::Hdr10 => HdrFormatDto::Hdr10,
            HdrFormat::Hdr10Plus => HdrFormatDto::Hdr10Plus,
            HdrFormat::DolbyVision => HdrFormatDto::DolbyVision,
            HdrFormat::Hlg => HdrFormatDto::Hlg,
        }
    }
}

impl From<AudioTrack> for AudioTrackDto {
    fn from(t: AudioTrack) -> Self {
        AudioTrackDto {
            index: t.index,
            codec: t.codec,
            channels: t.channels,
            language: t.language.map(|l| l.0),
            bitrate: t.bitrate,
        }
    }
}

impl From<EmbeddedSubtitleTrack> for SubtitleTrackDto {
    fn from(t: EmbeddedSubtitleTrack) -> Self {
        SubtitleTrackDto {
            index: t.index,
            language: t.language.map(|l| l.0),
            format: t.format.into(),
            forced: t.forced,
            default: t.default,
        }
    }
}

impl From<SubtitleFormat> for SubtitleFormatDto {
    fn from(f: SubtitleFormat) -> Self {
        match f {
            SubtitleFormat::Srt => SubtitleFormatDto::Srt,
            SubtitleFormat::Ass => SubtitleFormatDto::Ass,
            SubtitleFormat::Vtt => SubtitleFormatDto::Vtt,
            SubtitleFormat::Pgs => SubtitleFormatDto::Pgs,
            SubtitleFormat::VobSub => SubtitleFormatDto::VobSub,
        }
    }
}

impl From<SubtitleFile> for SubtitleFileDto {
    fn from(f: SubtitleFile) -> Self {
        SubtitleFileDto {
            id: f.id.0,
            language: f.language.map(|l| l.0),
            format: f.format.into(),
            source: f.source.into(),
        }
    }
}

impl From<SubtitleSource> for SubtitleSourceDto {
    fn from(s: SubtitleSource) -> Self {
        match s {
            SubtitleSource::OpenSubtitles => SubtitleSourceDto::OpenSubtitles,
            SubtitleSource::External => SubtitleSourceDto::External,
            SubtitleSource::Generated => SubtitleSourceDto::Generated,
            SubtitleSource::MachineTranslated => SubtitleSourceDto::MachineTranslated,
            SubtitleSource::Combined => SubtitleSourceDto::Combined,
        }
    }
}

impl From<Chapter> for ChapterDto {
    fn from(c: Chapter) -> Self {
        ChapterDto {
            title: c.title,
            start_ms: c.start_ms,
        }
    }
}

impl From<DetectedMarkers> for MarkersDto {
    fn from(m: DetectedMarkers) -> Self {
        MarkersDto {
            intro: m
                .intros
                .into_iter()
                .map(|i| MarkerDto {
                    start_ms: i.start_ms,
                    end_ms: i.end_ms,
                })
                .collect(),
            credits: m
                .credits
                .into_iter()
                .map(|c| MarkerDto {
                    start_ms: c.start_ms,
                    end_ms: c.end_ms,
                })
                .collect(),
        }
    }
}

impl From<TrickplayAsset> for TrickplayRefDto {
    fn from(a: TrickplayAsset) -> Self {
        TrickplayRefDto {
            interval_ms: a.interval_ms,
            columns: a.columns,
            rows: a.rows,
            tile_width: a.tile_width,
            tile_height: a.tile_height,
            sheets: a.sheet_paths.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{MovieId, TitleId, Version, VersionId};
    use domain::common::Quality;
    use domain::library::LibraryId;
    use jiff::Timestamp;

    fn detail() -> VersionDetail {
        VersionDetail {
            version: Version {
                id: VersionId("v1".into()),
                title: TitleId::Movie(MovieId("m1".into())),
                library: LibraryId("lib1".into()),
                quality: Quality::Fhd,
                container: "mkv".into(),
                path: "/media/v1.mkv".into(),
                size_bytes: 1,
                duration_ms: 1000,
                edition: None,
                available: true,
                added_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
            },
            video: Vec::new(),
            audio: Vec::new(),
            subtitles: Vec::new(),
            subtitle_files: Vec::new(),
            chapters: Vec::new(),
            markers: DetectedMarkers {
                intros: Vec::new(),
                credits: Vec::new(),
            },
            trickplay: Vec::new(),
        }
    }

    #[test]
    fn with_path_gates_path_on_admin() {
        let hidden = VersionDetailResponse::with_path(detail(), false);
        assert_eq!(hidden.path, None);
        assert!(
            !serde_json::to_string(&hidden)
                .unwrap()
                .contains("/media/v1.mkv")
        );

        let shown = VersionDetailResponse::with_path(detail(), true);
        assert_eq!(shown.path.as_deref(), Some("/media/v1.mkv"));
    }

    #[test]
    fn hdr_format_maps_every_variant() {
        for (domain, dto) in [
            (HdrFormat::Hdr10, HdrFormatDto::Hdr10),
            (HdrFormat::Hdr10Plus, HdrFormatDto::Hdr10Plus),
            (HdrFormat::DolbyVision, HdrFormatDto::DolbyVision),
            (HdrFormat::Hlg, HdrFormatDto::Hlg),
        ] {
            assert_eq!(
                serde_json::to_string(&HdrFormatDto::from(domain)).unwrap(),
                serde_json::to_string(&dto).unwrap()
            );
        }
    }

    #[test]
    fn subtitle_format_maps_every_variant() {
        for (domain, dto) in [
            (SubtitleFormat::Srt, SubtitleFormatDto::Srt),
            (SubtitleFormat::Ass, SubtitleFormatDto::Ass),
            (SubtitleFormat::Vtt, SubtitleFormatDto::Vtt),
            (SubtitleFormat::Pgs, SubtitleFormatDto::Pgs),
            (SubtitleFormat::VobSub, SubtitleFormatDto::VobSub),
        ] {
            assert_eq!(
                serde_json::to_string(&SubtitleFormatDto::from(domain)).unwrap(),
                serde_json::to_string(&dto).unwrap()
            );
        }
    }

    #[test]
    fn subtitle_source_maps_every_variant() {
        for (domain, dto) in [
            (
                SubtitleSource::OpenSubtitles,
                SubtitleSourceDto::OpenSubtitles,
            ),
            (SubtitleSource::External, SubtitleSourceDto::External),
            (SubtitleSource::Generated, SubtitleSourceDto::Generated),
            (
                SubtitleSource::MachineTranslated,
                SubtitleSourceDto::MachineTranslated,
            ),
            (SubtitleSource::Combined, SubtitleSourceDto::Combined),
        ] {
            assert_eq!(
                serde_json::to_string(&SubtitleSourceDto::from(domain)).unwrap(),
                serde_json::to_string(&dto).unwrap()
            );
        }
    }

    #[test]
    fn subtitle_file_maps_to_dto() {
        use domain::common::LanguageCode;
        use domain::media::{SubtitleFile, SubtitleFileId};

        let dto = SubtitleFileDto::from(SubtitleFile {
            id: SubtitleFileId("sf1".into()),
            version: VersionId("v1".into()),
            language: Some(LanguageCode("en".into())),
            format: SubtitleFormat::Srt,
            source: SubtitleSource::External,
            path: "/media/v1.en.srt".into(),
            translated_from: None,
        });
        assert_eq!(dto.id, "sf1");
        assert_eq!(dto.language.as_deref(), Some("en"));
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"source\":\"external\""));
        assert!(!json.contains("/media/v1.en.srt"));
    }
}

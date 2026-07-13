use serde::Serialize;

use domain::catalog::VersionDetail;
use domain::media::{
    AudioTrack, Chapter, DetectedMarkers, EmbeddedSubtitleTrack, HdrFormat, SubtitleFormat,
    TrickplayAsset, VideoTrack,
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
    pub video: Vec<VideoTrackDto>,
    pub audio: Vec<AudioTrackDto>,
    pub subtitles: Vec<SubtitleTrackDto>,
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
            video: d.video.into_iter().map(Into::into).collect(),
            audio: d.audio.into_iter().map(Into::into).collect(),
            subtitles: d.subtitles.into_iter().map(Into::into).collect(),
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
}

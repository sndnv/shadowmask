use crate::catalog::Version;
use crate::media::{
    AudioTrack, Chapter, DetectedMarkers, EmbeddedSubtitleTrack, TrickplayAsset, VideoTrack,
};

#[derive(Debug, Clone)]
pub struct VersionDetail {
    pub version: Version,
    pub video: Vec<VideoTrack>,
    pub audio: Vec<AudioTrack>,
    pub subtitles: Vec<EmbeddedSubtitleTrack>,
    pub chapters: Vec<Chapter>,
    pub markers: DetectedMarkers,
    pub trickplay: Vec<TrickplayAsset>,
}

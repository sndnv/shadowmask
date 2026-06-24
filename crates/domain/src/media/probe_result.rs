use crate::media::{AudioTrack, Chapter, EmbeddedSubtitleTrack, VideoTrack};

#[derive(Debug, Clone)]
pub struct ProbeResult {
    pub duration_ms: u64,
    pub video: Vec<VideoTrack>,
    pub audio: Vec<AudioTrack>,
    pub subtitles: Vec<EmbeddedSubtitleTrack>,
    pub chapters: Vec<Chapter>,
}

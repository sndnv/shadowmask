use crate::media::{AudioTrack, EmbeddedSubtitleTrack, VideoTrack};
use crate::profile::Container;
use crate::session::SubtitleSelection;

#[derive(Debug, Clone)]
pub struct NegotiationInput {
    pub container: Container,
    pub video: Vec<VideoTrack>,
    pub audio: Vec<AudioTrack>,
    pub subtitles: Vec<EmbeddedSubtitleTrack>,
    pub requested_audio: Option<u32>,
    pub requested_subtitle: Option<SubtitleSelection>,
    pub max_bitrate: Option<u64>,
    pub target_height: Option<u32>,
    pub force_burn: bool,
    pub downmix_stereo: bool,
}

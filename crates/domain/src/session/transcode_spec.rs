use crate::media::HdrFormat;
use crate::session::SessionId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscodeSpec {
    pub session: SessionId,
    pub input_path: String,
    pub duration_ms: u64,
    pub copy: bool,
    pub seek_ms: Option<u64>,
    pub audio_track: Option<u32>,
    pub max_height: Option<u32>,
    pub max_bitrate: Option<u64>,
    pub burn_subtitle_path: Option<String>,
    pub soft_subtitle: Option<SoftSubtitle>,
    pub downmix_stereo: bool,
    pub source_hdr: Option<HdrFormat>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoftSubtitle {
    pub source: SoftSubtitleSource,
    pub offset_ms: i64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoftSubtitleSource {
    Embedded(u32),
    File(String),
}

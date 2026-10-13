use crate::catalog::VersionId;
use crate::media::HdrFormat;
use crate::session::{SegmentContainer, SessionId, StreamGeneration};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscodeSpec {
    pub session: SessionId,
    pub generation: StreamGeneration,
    pub version: VersionId,
    pub input_path: String,
    pub duration_ms: u64,
    pub keyframes_us: Option<Vec<u64>>,
    pub copy: bool,
    pub container: SegmentContainer,
    pub seek_ms: Option<u64>,
    pub audio_track: Option<u32>,
    pub max_height: Option<u32>,
    pub max_bitrate: Option<u64>,
    pub burn_subtitle: Option<BurnSubtitle>,
    pub soft_subtitle: Option<SoftSubtitle>,
    pub convert_audio: bool,
    pub audio_channels: Option<u8>,
    pub source_hdr: Option<HdrFormat>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BurnSubtitle {
    File(String),
    EmbeddedText(u32),
    EmbeddedImage { index: u32, width: u32, height: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoftSubtitle {
    pub source: SoftSubtitleSource,
    pub offset_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoftSubtitleSource {
    Embedded(u32),
    File(String),
}

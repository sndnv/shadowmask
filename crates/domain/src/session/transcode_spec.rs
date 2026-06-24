use crate::session::SessionId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscodeSpec {
    pub session: SessionId,
    pub input_path: String,
    pub seek_ms: Option<u64>,
    pub audio_track: Option<u32>,
    pub max_height: Option<u32>,
    pub max_bitrate: Option<u64>,
    pub burn_subtitle_path: Option<String>,
}

use std::path::PathBuf;

use crate::session::SessionId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscodeStarted {
    pub session: SessionId,
    pub output_dir: String,
    pub origin_ms: u64,
    pub sequential: bool,
    pub subtitles: bool,
    pub subtitle_file: Option<PathBuf>,
}

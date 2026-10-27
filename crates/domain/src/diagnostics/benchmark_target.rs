use crate::catalog::{VersionDetail, VersionId};
use crate::media::HdrFormat;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkTarget {
    pub version: VersionId,
    pub path: String,
    pub duration_ms: u64,
    pub codec: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub hdr: Option<HdrFormat>,
    pub audio_track: Option<u32>,
    pub keyframes_us: Option<Vec<u64>>,
}

impl BenchmarkTarget {
    pub fn of(detail: &VersionDetail, keyframes_us: Option<Vec<u64>>) -> Self {
        let video = detail.video.first();
        Self {
            version: detail.version.id.clone(),
            path: detail.version.path.clone(),
            duration_ms: detail.version.duration_ms,
            codec: video.map(|track| track.codec.clone()),
            width: video.map(|track| track.width),
            height: video.map(|track| track.height),
            hdr: video.and_then(|track| track.hdr),
            audio_track: detail.audio.first().map(|track| track.index),
            keyframes_us: keyframes_us.filter(|keyframes| !keyframes.is_empty()),
        }
    }
}

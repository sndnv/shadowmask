use domain::diagnostics::FfmpegReport;
use serde::Serialize;

use crate::dto::server::FfmpegComponent;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FfmpegCapabilities {
    pub version: Option<String>,
    pub error: Option<String>,
    pub hwaccels: Vec<String>,
    pub encoders: Vec<FfmpegComponent>,
    pub filters: Vec<FfmpegComponent>,
}

impl From<FfmpegReport> for FfmpegCapabilities {
    fn from(ffmpeg: FfmpegReport) -> Self {
        Self {
            version: ffmpeg.version,
            error: ffmpeg.error,
            hwaccels: ffmpeg.hwaccels,
            encoders: ffmpeg.encoders.into_iter().map(FfmpegComponent::from).collect(),
            filters: ffmpeg.filters.into_iter().map(FfmpegComponent::from).collect(),
        }
    }
}

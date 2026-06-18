use crate::catalog::VersionId;
use crate::common::LanguageCode;

#[derive(Debug, Clone)]
pub struct AudioTrack {
    pub version: VersionId,
    pub index: u32,
    pub codec: String,
    pub channels: u8,
    pub language: Option<LanguageCode>,
    pub bitrate: Option<u64>,
}

use crate::common::LanguageCode;

#[derive(Debug, Clone)]
pub struct AudioTrack {
    pub index: u32,
    pub codec: String,
    pub channels: u8,
    pub language: Option<LanguageCode>,
    pub bitrate: Option<u64>,
}

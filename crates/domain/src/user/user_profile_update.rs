use crate::common::LanguageCode;
use crate::metadata::ContentRating;

#[derive(Debug, Clone, Default)]
pub struct UserProfileUpdate {
    pub preferred_audio: Option<Vec<LanguageCode>>,
    pub preferred_subtitle: Option<Vec<LanguageCode>>,
    pub max_content_rating: Option<ContentRating>,
    pub concurrent_stream_limit: Option<u32>,
    pub bitrate_cap: Option<u64>,
}

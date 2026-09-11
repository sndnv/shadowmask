use serde::Deserialize;

use domain::common::LanguageCode;
use domain::user::UserProfileUpdate;

use crate::dto::common::ContentRatingDto;

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub preferred_audio: Option<Vec<String>>,
    pub preferred_subtitle: Option<Vec<String>>,
    pub max_content_rating: Option<ContentRatingDto>,
    pub concurrent_stream_limit: Option<u32>,
    pub bitrate_cap: Option<u64>,
}

impl From<UpdateProfileRequest> for UserProfileUpdate {
    fn from(r: UpdateProfileRequest) -> Self {
        UserProfileUpdate {
            preferred_audio: r.preferred_audio.map(|v| v.into_iter().map(LanguageCode).collect()),
            preferred_subtitle: r
                .preferred_subtitle
                .map(|v| v.into_iter().map(LanguageCode).collect()),
            max_content_rating: r.max_content_rating.map(Into::into),
            concurrent_stream_limit: r.concurrent_stream_limit,
            bitrate_cap: r.bitrate_cap,
        }
    }
}

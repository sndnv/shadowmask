use jiff::Timestamp;

use crate::common::LanguageCode;
use crate::metadata::ContentRating;
use crate::user::Role;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserId(pub String);

#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub password_hash: String,
    pub role: Role,
    pub max_content_rating: Option<ContentRating>,
    pub preferred_audio: Vec<LanguageCode>,
    pub preferred_subtitle: Vec<LanguageCode>,
    pub concurrent_stream_limit: Option<u32>,
    pub bitrate_cap: Option<u64>,
    pub active: bool,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

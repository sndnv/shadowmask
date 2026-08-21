use serde::Serialize;

use domain::user::User;

use super::RoleDto;
use crate::dto::common::ContentRatingDto;

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub role: RoleDto,
    pub max_content_rating: Option<ContentRatingDto>,
    pub preferred_audio: Vec<String>,
    pub preferred_subtitle: Vec<String>,
    pub concurrent_stream_limit: Option<u32>,
    pub bitrate_cap: Option<u64>,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        UserResponse {
            id: u.id.0,
            username: u.username,
            role: u.role.into(),
            max_content_rating: u.max_content_rating.map(Into::into),
            preferred_audio: u.preferred_audio.into_iter().map(|l| l.0).collect(),
            preferred_subtitle: u.preferred_subtitle.into_iter().map(|l| l.0).collect(),
            concurrent_stream_limit: u.concurrent_stream_limit,
            bitrate_cap: u.bitrate_cap,
            active: u.active,
            created_at: u.created_at.to_string(),
            updated_at: u.updated_at.to_string(),
        }
    }
}

use serde::Serialize;

use domain::playback::Favorite;

use crate::dto::common::TitleRefDto;

#[derive(Debug, Serialize)]
pub struct FavoriteResponse {
    pub user_id: String,
    pub title: TitleRefDto,
    pub added_at: String,
}

impl From<Favorite> for FavoriteResponse {
    fn from(f: Favorite) -> Self {
        FavoriteResponse {
            user_id: f.user.0,
            title: f.title.into(),
            added_at: f.added_at.to_string(),
        }
    }
}

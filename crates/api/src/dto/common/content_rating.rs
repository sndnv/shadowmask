use serde::{Deserialize, Serialize};

use domain::metadata::ContentRating;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentRatingDto {
    pub system: String,
    pub code: String,
}

impl From<ContentRating> for ContentRatingDto {
    fn from(r: ContentRating) -> Self {
        ContentRatingDto {
            system: r.system,
            code: r.code,
        }
    }
}

impl From<ContentRatingDto> for ContentRating {
    fn from(r: ContentRatingDto) -> Self {
        ContentRating {
            system: r.system,
            code: r.code,
        }
    }
}

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
            preferred_audio: r.preferred_audio.map(canonical),
            preferred_subtitle: r.preferred_subtitle.map(canonical),
            max_content_rating: r.max_content_rating.map(Into::into),
            concurrent_stream_limit: r.concurrent_stream_limit,
            bitrate_cap: r.bitrate_cap,
        }
    }
}

fn canonical(codes: Vec<String>) -> Vec<LanguageCode> {
    codes.iter().map(|code| LanguageCode::canonical(code.trim())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferred_languages_are_stored_in_their_one_form() {
        let request: UpdateProfileRequest = serde_json::from_str(
            r#"{"preferred_audio":["ENG"," ja "],"preferred_subtitle":["pt_br"]}"#,
        )
        .unwrap();

        let update = UserProfileUpdate::from(request);

        let codes = |list: Option<Vec<LanguageCode>>| {
            list.unwrap().into_iter().map(|code| code.0).collect::<Vec<_>>()
        };
        assert_eq!(codes(update.preferred_audio), ["en", "ja"]);
        assert_eq!(codes(update.preferred_subtitle), ["pt-BR"]);
    }
}

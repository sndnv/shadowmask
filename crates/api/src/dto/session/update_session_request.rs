use serde::Deserialize;

use domain::session::SessionUpdate;

use super::SubtitleChangeDto;

#[derive(Debug, Deserialize)]
pub struct UpdateSessionRequest {
    pub audio_track: Option<u32>,
    #[serde(default)]
    pub subtitle: SubtitleChangeDto,
    pub target_height: Option<u32>,
    #[serde(default)]
    pub force_burn: bool,
    #[serde(default)]
    pub downmix_stereo: bool,
}

impl From<UpdateSessionRequest> for SessionUpdate {
    fn from(u: UpdateSessionRequest) -> Self {
        SessionUpdate {
            audio_track: u.audio_track,
            subtitle: u.subtitle.into(),
            target_height: u.target_height,
            force_burn: u.force_burn,
            downmix_stereo: u.downmix_stereo,
        }
    }
}

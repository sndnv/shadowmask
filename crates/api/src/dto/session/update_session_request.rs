use serde::Deserialize;

use domain::session::SessionUpdate;

use super::SubtitleChangeDto;

#[derive(Debug, Deserialize)]
pub struct UpdateSessionRequest {
    pub audio_track: Option<u32>,
    pub subtitle: SubtitleChangeDto,
}

impl From<UpdateSessionRequest> for SessionUpdate {
    fn from(u: UpdateSessionRequest) -> Self {
        SessionUpdate {
            audio_track: u.audio_track,
            subtitle: u.subtitle.into(),
        }
    }
}

use crate::session::{DeliveryPreference, SubtitleChange};

#[derive(Debug, Clone)]
pub struct SessionUpdate {
    pub audio_track: Option<u32>,
    pub subtitle: SubtitleChange,
    pub target_height: Option<u32>,
    pub force_burn: bool,
    pub downmix_stereo: bool,
    pub delivery: DeliveryPreference,
}

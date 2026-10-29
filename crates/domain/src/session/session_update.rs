use crate::session::{DeliveryPreference, SoundSettings, SubtitleChange};

#[derive(Debug, Clone)]
pub struct SessionUpdate {
    pub position_ms: u64,
    pub audio_track: Option<u32>,
    pub subtitle: SubtitleChange,
    pub target_height: Option<u32>,
    pub force_burn: bool,
    pub downmix_stereo: bool,
    pub sound: SoundSettings,
    pub delivery: DeliveryPreference,
}

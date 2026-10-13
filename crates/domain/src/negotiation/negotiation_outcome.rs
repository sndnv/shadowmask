use crate::negotiation::NegotiationReason;
use crate::session::{DeliveryMode, SelectedTracks};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegotiationOutcome {
    pub mode: DeliveryMode,
    pub selected: SelectedTracks,
    pub reasons: Vec<NegotiationReason>,
    pub convert_audio: bool,
    pub audio_channels: Option<u8>,
}

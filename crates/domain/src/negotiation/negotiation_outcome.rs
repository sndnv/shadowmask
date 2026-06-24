use crate::session::{DeliveryMode, SelectedTracks};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegotiationOutcome {
    pub mode: DeliveryMode,
    pub selected: SelectedTracks,
}

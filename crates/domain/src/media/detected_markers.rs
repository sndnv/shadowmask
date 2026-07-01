use crate::media::{CreditsMarker, IntroMarker};

#[derive(Debug, Clone, Default)]
pub struct DetectedMarkers {
    pub intros: Vec<IntroMarker>,
    pub credits: Vec<CreditsMarker>,
}

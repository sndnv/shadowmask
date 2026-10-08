use domain::diagnostics::Presence;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FfmpegComponent {
    pub name: String,
    pub present: bool,
}

impl From<Presence> for FfmpegComponent {
    fn from(presence: Presence) -> Self {
        Self { name: presence.name, present: presence.present }
    }
}

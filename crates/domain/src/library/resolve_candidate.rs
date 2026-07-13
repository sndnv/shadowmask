use crate::library::ResolveTarget;
use crate::metadata::MediaKind;

#[derive(Debug, Clone)]
pub struct ResolveCandidate {
    pub target: ResolveTarget,
    pub title: String,
    pub year: Option<u16>,
    pub kind: MediaKind,
}

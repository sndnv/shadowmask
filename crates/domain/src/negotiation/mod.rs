mod negotiate;
mod negotiation_input;
mod negotiation_outcome;
mod negotiation_reason;
mod preference;
mod resolution;

pub use negotiate::{effective_max_height, negotiate};
pub use negotiation_input::NegotiationInput;
pub use negotiation_outcome::NegotiationOutcome;
pub use negotiation_reason::NegotiationReason;
pub use preference::{preferred_audio_track, preferred_subtitle_track};
pub use resolution::{
    AvailableSubtitles, ResolvedAudio, ResolvedSubtitle, resolve_audio, resolve_subtitle,
};

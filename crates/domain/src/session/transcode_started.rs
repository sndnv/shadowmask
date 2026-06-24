use crate::session::SessionId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscodeStarted {
    pub session: SessionId,
    pub output_dir: String,
}

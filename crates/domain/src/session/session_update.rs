use crate::session::SubtitleChange;

#[derive(Debug, Clone)]
pub struct SessionUpdate {
    pub audio_track: Option<u32>,
    pub subtitle: SubtitleChange,
}

use crate::session::SubtitleSelection;

#[derive(Debug, Clone)]
pub enum SubtitleChange {
    Keep,
    Disable,
    Set(SubtitleSelection),
}

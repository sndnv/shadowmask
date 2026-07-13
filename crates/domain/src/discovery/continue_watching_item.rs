use crate::playback::{PlaybackProgress, ResumeCard};

#[derive(Debug, Clone)]
pub struct ContinueWatchingItem {
    pub progress: PlaybackProgress,
    pub card: ResumeCard,
}

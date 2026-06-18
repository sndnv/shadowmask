use crate::playback::PlaybackProgress;

#[derive(Debug, Clone)]
pub struct ContinueWatchingItem {
    pub progress: PlaybackProgress,
}

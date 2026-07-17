use crate::playback::WatchTarget;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchedRollup {
    pub target: WatchTarget,
    pub watched: bool,
    pub completed: bool,
    pub watched_episodes: u32,
    pub total_episodes: u32,
}

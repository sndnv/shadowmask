use crate::playback::ResumeCard;
use crate::session::PlaybackSession;

#[derive(Debug, Clone)]
pub struct NowPlaying {
    pub session: PlaybackSession,
    pub card: ResumeCard,
}

use serde::{Deserialize, Serialize};

use domain::session::PlaybackState;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackStateDto {
    Playing,
    Paused,
}

impl From<PlaybackState> for PlaybackStateDto {
    fn from(s: PlaybackState) -> Self {
        match s {
            PlaybackState::Playing => PlaybackStateDto::Playing,
            PlaybackState::Paused => PlaybackStateDto::Paused,
        }
    }
}

impl From<PlaybackStateDto> for PlaybackState {
    fn from(s: PlaybackStateDto) -> Self {
        match s {
            PlaybackStateDto::Playing => PlaybackState::Playing,
            PlaybackStateDto::Paused => PlaybackState::Paused,
        }
    }
}

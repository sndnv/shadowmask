use serde::Serialize;

use crate::dto::catalog::{EpisodeResponse, MovieResponse};
use crate::dto::discovery::ContinueWatchingItemResponse;
use crate::dto::session::PlaybackSessionResponse;

#[derive(Debug, Serialize)]
pub struct ContinueResponse {
    pub now_playing: Vec<PlaybackSessionResponse>,
    pub in_progress: Vec<ContinueWatchingItemResponse>,
    pub next_episodes: Vec<EpisodeResponse>,
    pub next_movies: Vec<MovieResponse>,
}

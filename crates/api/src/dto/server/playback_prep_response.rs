use domain::diagnostics::LibraryPrep;
use serde::Serialize;

use crate::dto::server::LibraryPrepStatus;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlaybackPrepResponse {
    pub ffmpeg: Option<String>,
    pub libraries: Vec<LibraryPrepStatus>,
}

impl PlaybackPrepResponse {
    pub fn new(ffmpeg: Option<String>, libraries: Vec<LibraryPrep>) -> Self {
        Self { ffmpeg, libraries: libraries.into_iter().map(Into::into).collect() }
    }
}

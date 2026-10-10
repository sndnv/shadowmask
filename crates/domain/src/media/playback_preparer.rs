use std::future::Future;

use crate::catalog::VersionId;
use crate::error::PlaybackPrepError;
use crate::media::{EmbeddedSubtitleTrack, PlaybackPrep};

pub trait PlaybackPreparer {
    fn prepare(
        &self,
        version: &VersionId,
        input_path: &str,
        subtitles: &[EmbeddedSubtitleTrack],
    ) -> impl Future<Output = Result<PlaybackPrep, PlaybackPrepError>> + Send;
}

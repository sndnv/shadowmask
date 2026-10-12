use std::future::Future;

use crate::diagnostics::LibraryPrep;
use crate::error::RepositoryError;

pub trait PlaybackPrepAudit: Send + Sync {
    fn libraries(
        &self,
        ffmpeg: Option<&str>,
    ) -> impl Future<Output = Result<Vec<LibraryPrep>, RepositoryError>> + Send;
}

use std::future::Future;

use crate::catalog::VersionId;
use crate::error::SubtitleError;
use crate::media::SubtitleFormat;

pub trait SubtitleStore {
    fn store(
        &self,
        version: &VersionId,
        file_id: &str,
        format: SubtitleFormat,
        content: &str,
    ) -> impl Future<Output = Result<String, SubtitleError>> + Send;
}

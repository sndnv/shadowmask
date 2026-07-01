use std::future::Future;

use crate::error::SubtitleError;
use crate::media::{FetchedSubtitle, SubtitleCandidate, SubtitleQuery};

pub trait SubtitleProvider {
    fn search(
        &self,
        query: &SubtitleQuery,
    ) -> impl Future<Output = Result<Vec<SubtitleCandidate>, SubtitleError>> + Send;

    fn download(
        &self,
        file_id: &str,
    ) -> impl Future<Output = Result<FetchedSubtitle, SubtitleError>> + Send;
}

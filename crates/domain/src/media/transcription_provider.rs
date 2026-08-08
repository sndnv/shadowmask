use std::future::Future;

use crate::error::TranscriptionError;
use crate::media::{FetchedSubtitle, TranscriptionSpec};

pub trait TranscriptionProvider {
    fn transcribe(
        &self,
        request: &TranscriptionSpec,
    ) -> impl Future<Output = Result<FetchedSubtitle, TranscriptionError>> + Send;
}

use std::future::Future;

use crate::error::TranscriptionError;
use crate::media::{FetchedSubtitle, TranscriptionRequest};

pub trait TranscriptionProvider {
    fn transcribe(
        &self,
        request: &TranscriptionRequest,
    ) -> impl Future<Output = Result<FetchedSubtitle, TranscriptionError>> + Send;
}

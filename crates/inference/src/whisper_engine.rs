use std::future::Future;

use domain::error::TranscriptionError;

use crate::Segment;

pub trait WhisperEngine {
    fn transcribe(
        &self,
        samples: Vec<f32>,
        language: Option<String>,
    ) -> impl Future<Output = Result<Vec<Segment>, TranscriptionError>> + Send;
}

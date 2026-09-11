use domain::error::TranscriptionError;
use domain::media::{FetchedSubtitle, TranscriptionProvider, TranscriptionSpec};

#[derive(Debug, Default, Clone, Copy)]
pub struct DisabledTranscriptionProvider;

impl TranscriptionProvider for DisabledTranscriptionProvider {
    async fn transcribe(
        &self,
        _request: &TranscriptionSpec,
    ) -> Result<FetchedSubtitle, TranscriptionError> {
        Err(TranscriptionError::Unsupported("transcription feature not built".to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn transcribe_is_unsupported() {
        let request = TranscriptionSpec {
            audio_path: "/media/v1.mkv".to_owned(),
            source_language: None,
            audio_track_index: None,
        };
        let err = DisabledTranscriptionProvider.transcribe(&request).await.unwrap_err();
        assert!(matches!(err, TranscriptionError::Unsupported(_)));
    }
}

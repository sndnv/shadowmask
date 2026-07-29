use crate::common::LanguageCode;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TranscriptionRequest {
    pub audio_path: String,
    pub source_language: Option<LanguageCode>,
    pub audio_track_index: Option<u32>,
}

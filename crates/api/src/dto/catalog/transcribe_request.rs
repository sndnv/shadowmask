use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TranscribeRequest {
    #[serde(default)]
    pub audio_track_index: Option<u32>,
    #[serde(default)]
    pub source_language: Option<String>,
}

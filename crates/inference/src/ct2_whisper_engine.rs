use std::path::PathBuf;

use ct2rs::{Config, Whisper, WhisperOptions};
use domain::error::TranscriptionError;

use crate::whisper_parse::parse_segments;
use crate::{Segment, WhisperEngine};

pub struct Ct2WhisperEngine {
    model_path: PathBuf,
}

impl Ct2WhisperEngine {
    pub fn new(model_path: impl Into<PathBuf>) -> Self {
        Self { model_path: model_path.into() }
    }
}

impl WhisperEngine for Ct2WhisperEngine {
    async fn transcribe(
        &self,
        samples: Vec<f32>,
        language: Option<String>,
    ) -> Result<Vec<Segment>, TranscriptionError> {
        let model_path = self.model_path.clone();
        tokio::task::spawn_blocking(move || {
            let whisper = Whisper::new(&model_path, Config::default())
                .map_err(|e| TranscriptionError::Backend(e.to_string()))?;
            let options = WhisperOptions {
                beam_size: 1,
                no_repeat_ngram_size: 3,
                repetition_penalty: 1.2,
                ..WhisperOptions::default()
            };
            let outputs = whisper
                .generate(&samples, language.as_deref(), true, &options)
                .map_err(|e| TranscriptionError::Backend(e.to_string()))?;
            let chunk_seconds = whisper.n_samples() as f64 / whisper.sampling_rate() as f64;
            Ok(parse_segments(&outputs, chunk_seconds))
        })
        .await
        .map_err(|e| TranscriptionError::Backend(e.to_string()))?
    }
}

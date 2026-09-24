use std::path::PathBuf;
use std::sync::Arc;

use ct2rs::{Config, Whisper, WhisperOptions};
use domain::error::TranscriptionError;
use tokio::sync::OnceCell;

use crate::whisper_parse::parse_segments;
use crate::{Segment, WhisperEngine};

pub struct Ct2WhisperEngine {
    model_path: PathBuf,
    threads: usize,
    whisper: OnceCell<Arc<Whisper>>,
}

impl Ct2WhisperEngine {
    pub fn new(model_path: impl Into<PathBuf>, threads: usize) -> Self {
        Self { model_path: model_path.into(), threads, whisper: OnceCell::new() }
    }

    async fn whisper(&self) -> Result<Arc<Whisper>, TranscriptionError> {
        self.whisper
            .get_or_try_init(|| async {
                let model_path = self.model_path.clone();
                let config = Config { num_threads_per_replica: self.threads, ..Config::default() };
                #[rustfmt::skip]
                tracing::info!("loading the transcription model at [{}] with [{}] thread(s)", model_path.display(), self.threads);
                tokio::task::spawn_blocking(move || {
                    Whisper::new(&model_path, config)
                        .map(Arc::new)
                        .map_err(|e| TranscriptionError::Backend(e.to_string()))
                })
                .await
                .map_err(|e| TranscriptionError::Backend(e.to_string()))?
            })
            .await
            .map(Arc::clone)
    }
}

impl WhisperEngine for Ct2WhisperEngine {
    async fn transcribe(
        &self,
        samples: Vec<f32>,
        language: Option<String>,
    ) -> Result<Vec<Segment>, TranscriptionError> {
        let whisper = self.whisper().await?;
        tokio::task::spawn_blocking(move || {
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

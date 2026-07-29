use std::path::PathBuf;

use ct2rs::{Config, Whisper, WhisperOptions};
use domain::error::TranscriptionError;

use crate::{Segment, WhisperEngine};

pub struct Ct2WhisperEngine {
    model_path: PathBuf,
}

impl Ct2WhisperEngine {
    pub fn new(model_path: impl Into<PathBuf>) -> Self {
        Self {
            model_path: model_path.into(),
        }
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
            Ok(parse_segments(&outputs))
        })
        .await
        .map_err(|e| TranscriptionError::Backend(e.to_string()))?
    }
}

fn parse_segments(outputs: &[String]) -> Vec<Segment> {
    let mut segments = Vec::new();
    for line in outputs {
        let stamps: Vec<f64> = timestamps(line);
        let text = strip_timestamps(line);
        if text.is_empty() {
            continue;
        }
        let start = stamps.first().copied().unwrap_or(0.0);
        let end = stamps.last().copied().unwrap_or(start);
        segments.push(Segment {
            start_ms: (start * 1000.0) as u64,
            end_ms: (end * 1000.0) as u64,
            text,
        });
    }
    segments
}

fn timestamps(line: &str) -> Vec<f64> {
    line.match_indices("<|")
        .filter_map(|(i, _)| {
            line[i + 2..]
                .split_once("|>")
                .and_then(|(token, _)| token.parse::<f64>().ok())
        })
        .collect()
}

fn strip_timestamps(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(open) = rest.find("<|") {
        out.push_str(&rest[..open]);
        match rest[open..].find("|>") {
            Some(close) => rest = &rest[open + close + 2..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out.trim().to_owned()
}

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::enrichment::{TranscriptionConfig, TranslationConfig, UpscalingConfig};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EnrichmentConfig {
    pub model_cache: PathBuf,
    pub concurrency: usize,
    pub transcription: TranscriptionConfig,
    pub translation: TranslationConfig,
    pub upscaling: UpscalingConfig,
}

impl Default for EnrichmentConfig {
    fn default() -> Self {
        Self {
            model_cache: PathBuf::from("data/enrichment-models"),
            concurrency: 1,
            transcription: TranscriptionConfig::default(),
            translation: TranslationConfig::default(),
            upscaling: UpscalingConfig::default(),
        }
    }
}

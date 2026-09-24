use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::enrichment::{TranscriptionConfig, TranslationConfig, UpscalingConfig};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EnrichmentConfig {
    pub model_cache: PathBuf,
    pub threads: Option<usize>,
    pub transcription: TranscriptionConfig,
    pub translation: TranslationConfig,
    pub upscaling: UpscalingConfig,
}

impl Default for EnrichmentConfig {
    fn default() -> Self {
        Self {
            model_cache: PathBuf::from("data/enrichment-models"),
            threads: None,
            transcription: TranscriptionConfig::default(),
            translation: TranslationConfig::default(),
            upscaling: UpscalingConfig::default(),
        }
    }
}

pub fn resolve_enrichment_threads(configured: Option<usize>, available: usize) -> usize {
    match configured {
        Some(threads) => threads.max(1),
        None => (available / 2).max(1),
    }
}

pub fn available_cores() -> usize {
    std::thread::available_parallelism().map(usize::from).unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_threads_leave_half_the_cores_for_everything_else() {
        assert_eq!(resolve_enrichment_threads(None, 12), 6);
        assert_eq!(resolve_enrichment_threads(None, 4), 2);
        assert_eq!(resolve_enrichment_threads(None, 1), 1);
        assert_eq!(resolve_enrichment_threads(None, 0), 1);
    }

    #[test]
    fn a_configured_thread_count_wins_but_never_reaches_zero() {
        assert_eq!(resolve_enrichment_threads(Some(3), 12), 3);
        assert_eq!(resolve_enrichment_threads(Some(32), 12), 32);
        assert_eq!(resolve_enrichment_threads(Some(0), 12), 1);
    }

    #[test]
    fn the_machine_always_reports_at_least_one_core() {
        assert!(available_cores() >= 1);
    }
}
